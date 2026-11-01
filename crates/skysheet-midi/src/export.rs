use crate::{
    Failure, Report, structure,
    timing::{dyadic, round_ratio},
};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use num_traits::ToPrimitive;
use skysheet_score::Score;

pub fn export(score: &Score) -> Result<(Vec<u8>, Report), Failure> {
    let mut report = Report::new();
    report.track_count = score.tracks.len();
    report.note_count = score.tracks.iter().map(|t| t.notes.len()).sum();
    if let Err(e) = score.validate() {
        report.push("error", "score_validation", e.to_string(), None);
        return Err(report.failure());
    }
    report.reference_tempo = Some(score.timeline.tempo);
    if score.tracks.len() >= u16::MAX as usize {
        report.push(
            "error",
            "midi_track_range",
            "track count including conductor exceeds 65535",
            None,
        );
    }
    if score.timeline.time_signature.numerator > 255 {
        report.push(
            "error",
            "midi_meter_range",
            "MIDI numerator exceeds one byte (255)",
            None,
        );
    }
    // Conservative per-event space budget, checked before materializing event lists.
    let estimated = score.tracks.iter().try_fold(64_usize, |size, t| {
        size.checked_add(t.name.len())?
            .checked_add(t.notes.len().checked_mul(32)?)?
            .checked_add(32)
    });
    if estimated.is_none_or(|size| size > structure::MAX_FILE_BYTES) {
        report.push(
            "error",
            "export_resource_limit",
            "estimated encoding exceeds this converter's 32 MiB resource budget",
            None,
        );
    }
    let (bn, bd) = dyadic(score.timeline.tempo).expect("validated tempo");
    let (micros, fractional) = round_ratio(&(bd * 60_000_000_u32), &bn);
    let micros = match micros.to_u32().filter(|n| (1..=0xffffff).contains(n)) {
        Some(micros) => micros,
        None => {
            report.push(
                "error",
                "midi_tempo_range",
                "rounded microseconds/quarter outside 1..16777215; Score tempo remains valid",
                None,
            );
            return Err(report.failure());
        }
    };
    report.output_microseconds_per_quarter = Some(micros);
    if fractional {
        report.push("warning", "tempo_approximated", format!("Score BPM encoded as nearest integer {micros} microseconds/quarter; elapsed seconds may differ"), None);
    }
    if report.failed() {
        return Err(report.failure());
    }

    // A channel/key pair is never active twice, even across SMF tracks. This
    // prevents receiver-wide note-off ambiguity for nested/crossing unisons.
    let mut ends = [[0_u64; 128]; 16];
    // Reserve each channel/key to its first Score track for the whole file.
    // Otherwise a same-tick onset on an earlier track could precede the old
    // track's note-off when an external player merges tracks.
    let mut owners = [[None; 128]; 16];
    let mut channels: Vec<Vec<u8>> = score
        .tracks
        .iter()
        .map(|t| vec![0; t.notes.len()])
        .collect();
    let mut order: Vec<_> = score
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(ti, t)| {
            t.notes
                .iter()
                .enumerate()
                .map(move |(ni, n)| (n.start_tick, ti, ni, n))
        })
        .collect();
    order.sort_by_key(|(start, ti, ni, _)| (*start, *ti, *ni));
    for (start, ti, ni, n) in order {
        let key = usize::from(n.pitch.midi_number());
        let Some(channel) = (0..16).find(|c| {
            *c != 9 && ends[*c][key] <= start && owners[*c][key].is_none_or(|owner| owner == ti)
        }) else {
            report.push(
                "error",
                "midi_channel_capacity",
                format!(
                    "note {}: all 15 melodic channels occupied for MIDI key {key}",
                    n.id
                ),
                None,
            );
            return Err(report.failure());
        };
        ends[channel][key] = start
            .checked_add(n.duration_ticks)
            .expect("validated note end");
        owners[channel][key] = Some(ti);
        channels[ti][ni] = channel as u8;
    }
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(score.timeline.ppq.into()),
    ));
    smf.tracks.push(vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(micros.into())),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                score.timeline.time_signature.numerator as u8,
                score.timeline.time_signature.denominator.trailing_zeros() as u8,
                24,
                8,
            )),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]);
    for (ti, track) in score.tracks.iter().enumerate() {
        let mut events = Vec::new();
        for (ni, n) in track.notes.iter().enumerate() {
            let channel = channels[ti][ni].into();
            let key = n.pitch.midi_number().into();
            events.push((
                n.start_tick,
                1_u8,
                ni,
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOn {
                        key,
                        vel: 64.into(),
                    },
                },
            ));
            events.push((
                n.start_tick + n.duration_ticks,
                0_u8,
                ni,
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOff { key, vel: 0.into() },
                },
            ));
        }
        events.sort_by_key(|e| (e.0, e.1, e.2));
        let mut encoded = vec![TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
        }];
        let mut previous = 0;
        for (tick, _, _, kind) in events {
            let delta = tick - previous;
            if delta > 0x0fff_ffff {
                report.push(
                    "error",
                    "midi_delta_range",
                    format!(
                        "track {ti}: delta {delta} exceeds four-byte VLQ; no truncation applied"
                    ),
                    None,
                );
                return Err(report.failure());
            }
            encoded.push(TrackEvent {
                delta: (delta as u32).into(),
                kind,
            });
            previous = tick;
        }
        encoded.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        smf.tracks.push(encoded);
        let mut used = channels[ti].clone();
        used.sort_unstable();
        used.dedup();
        report.push(
            "info",
            "channels_assigned",
            format!(
                "track {ti}: MIDI channels {:?} (1-based)",
                used.iter().map(|c| c + 1).collect::<Vec<_>>()
            ),
            None,
        );
    }
    let mut bytes = Vec::new();
    if let Err(e) = smf.write_std(&mut bytes) {
        report.push("error", "midi_encoding", e.to_string(), None);
        return Err(report.failure());
    }
    // Our resource budget is below both SMF chunk u32 and meta-length u28 limits.
    if let Err(e) = structure::validate(&bytes) {
        report.push("error", "midi_encoding", e, None);
        return Err(report.failure());
    }
    report.push("warning", "score_metadata_omitted", "Score IDs, role, text, pitch spelling and instrument behavior are not encoded; Note On velocity 64 / Note Off velocity 0", None);
    Ok((bytes, report))
}
