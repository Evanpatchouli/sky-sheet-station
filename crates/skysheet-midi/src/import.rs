use crate::{
    Failure, MetadataEvent, MeterEvent, Report, SourceNote, TempoEvent, structure, timing::Timing,
};
use midly::{MetaMessage, MidiMessage, Smf, Timing as Division, TrackEventKind};
use skysheet_score::{
    Instrument, Metadata, Note, PPQ, Pitch, PlaybackMode, SCORE_FORMAT_VERSION, SKY_15_KEYS, Score,
    TimeSignature, Timeline, Track, TrackRole,
};
use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
pub struct ImportOptions {
    pub reference_tempo: Option<f64>,
    pub name: Option<String>,
}

fn pitch(key: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        NAMES[usize::from(key % 12)],
        i16::from(key / 12) - 1
    )
}

/// Inspect reliable SMF structure and all source events. Semantic conversion
/// problems appear as report diagnostics; no Score is manufactured by inspection.
pub fn inspect(bytes: &[u8]) -> Result<Report, Failure> {
    scan(bytes).map(|(_, report)| report)
}

type Pending = BTreeMap<(u8, u8), VecDeque<(u64, usize, u8)>>;

fn scan(bytes: &[u8]) -> Result<(Vec<String>, Report), Failure> {
    let mut report = Report::new();
    if let Err(e) = structure::validate(bytes) {
        report.push("error", "midi_structure", e, None);
        return Err(report.failure());
    }
    let smf = match Smf::parse(bytes) {
        Ok(smf) => smf,
        Err(e) => {
            report.push("error", "midi_parse", e.to_string(), None);
            return Err(report.failure());
        }
    };
    report.format = Some(if smf.header.format == midly::Format::SingleTrack {
        0
    } else {
        1
    });
    let Division::Metrical(ppq) = smf.header.timing else {
        unreachable!("structure checked PPQ")
    };
    report.source_ppq = Some(ppq.as_int());
    report.track_count = smf.tracks.len();
    let mut names = Vec::new();
    for (track, events) in smf.tracks.iter().enumerate() {
        let mut pending: Pending = BTreeMap::new();
        let mut tick = 0_u64;
        let mut name = None;
        for (event, value) in events.iter().enumerate() {
            let Some(next) = tick.checked_add(u64::from(value.delta.as_int())) else {
                report.push(
                    "error",
                    "source_tick_overflow",
                    "absolute source tick overflow",
                    Some((track, event, tick)),
                );
                return Err(report.failure());
            };
            tick = next;
            let location = Some((track, event, tick));
            match value.kind {
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() != 0 => {
                            let queue = pending.entry((ch, key.as_int())).or_default();
                            if !queue.is_empty() {
                                report.push(
                                    "warning",
                                    "overlapping_note_on",
                                    format!(
                                        "channel {} key {}: overlapping onsets paired FIFO",
                                        ch + 1,
                                        key.as_int()
                                    ),
                                    location,
                                );
                            }
                            queue.push_back((tick, event, vel.as_int()));
                        }
                        MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                            if let Some((start, on_event, velocity)) = pending
                                .get_mut(&(ch, key.as_int()))
                                .and_then(VecDeque::pop_front)
                            {
                                if tick <= start {
                                    report.push("error", "nonpositive_source_duration", format!("channel {} key {}: end does not follow onset event {on_event}", ch + 1, key.as_int()), location);
                                }
                                report.source_notes.push(SourceNote {
                                    track,
                                    channel: ch,
                                    key: key.as_int(),
                                    pitch: pitch(key.as_int()),
                                    velocity,
                                    on_event,
                                    off_event: event,
                                    start_tick: start,
                                    end_tick: tick,
                                });
                            } else {
                                report.push(
                                    "error",
                                    "unmatched_note_off",
                                    format!(
                                        "channel {} key {} has no pending onset",
                                        ch + 1,
                                        key.as_int()
                                    ),
                                    location,
                                );
                            }
                        }
                        MidiMessage::ProgramChange { program } => {
                            report.metadata.push(MetadataEvent {
                                track,
                                event,
                                tick,
                                kind: "program_change".into(),
                                data: vec![ch, program.as_int()],
                            });
                            report.push("warning", "program_omitted", format!("channel {} program {} retained in report; Score uses target instrument", ch + 1, program.as_int()), location);
                        }
                        // These change sounding pitch, articulation or voice state. No
                        // controller engine is implied by preserving note messages.
                        other => report.push(
                            "error",
                            "unsupported_midi_event",
                            format!(
                                "channel {}: {other:?} cannot be faithfully represented",
                                ch + 1
                            ),
                            location,
                        ),
                    }
                }
                TrackEventKind::Meta(MetaMessage::Tempo(value)) => {
                    report.tempos.push(TempoEvent {
                        track,
                        event,
                        tick,
                        microseconds_per_quarter: value.as_int(),
                    });
                    if value.as_int() == 0 {
                        report.push("error", "zero_tempo", "source tempo is zero", location);
                    }
                    if track != 0 {
                        report.push(
                            "warning",
                            "tempo_outside_first_track",
                            "tempo retained in the global map",
                            location,
                        );
                    }
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, c, q)) => {
                    report.meters.push(MeterEvent {
                        track,
                        event,
                        tick,
                        numerator: n,
                        denominator_power: d,
                        clocks_per_click: c,
                        thirty_seconds_per_quarter: q,
                    });
                    if n == 0 {
                        report.push(
                            "error",
                            "invalid_meter",
                            "time signature numerator is zero",
                            location,
                        );
                    }
                    if q != 8 {
                        report.push(
                            "error",
                            "unsupported_notation_unit",
                            format!("expected 8 notated 32nd notes per MIDI quarter, got {q}"),
                            location,
                        );
                    }
                    report.push(
                        "warning",
                        "meter_click_omitted",
                        format!("metronome click parameter {c} retained only in report"),
                        location,
                    );
                }
                TrackEventKind::Meta(MetaMessage::EndOfTrack) => {}
                TrackEventKind::Meta(meta) => {
                    let (kind, data) = match meta {
                        MetaMessage::TrackName(data) => {
                            if std::str::from_utf8(data).is_err() {
                                report.push("warning", "metadata_encoding", "track name is not UTF-8; replacement characters displayed, original bytes retained", location);
                            }
                            if name.is_none() && !data.is_empty() {
                                name = Some(String::from_utf8_lossy(data).into_owned());
                            }
                            ("track_name".into(), data.to_vec())
                        }
                        MetaMessage::Text(data)
                        | MetaMessage::Copyright(data)
                        | MetaMessage::InstrumentName(data)
                        | MetaMessage::Lyric(data)
                        | MetaMessage::Marker(data)
                        | MetaMessage::CuePoint(data)
                        | MetaMessage::ProgramName(data)
                        | MetaMessage::DeviceName(data) => (format!("{meta:?}"), data.to_vec()),
                        MetaMessage::Unknown(kind, data) => {
                            report.push(
                                "error",
                                "unsupported_meta_event",
                                format!("unknown meta {kind:#x}"),
                                location,
                            );
                            (format!("unknown_{kind:02x}"), data.to_vec())
                        }
                        MetaMessage::SequencerSpecific(data) => {
                            report.push(
                                "error",
                                "unsupported_meta_event",
                                "sequencer-specific interpretation is unsupported",
                                location,
                            );
                            ("sequencer_specific".into(), data.to_vec())
                        }
                        MetaMessage::SmpteOffset(_) | MetaMessage::MidiPort(_) => {
                            report.push(
                                "error",
                                "unsupported_meta_event",
                                format!("{meta:?} changes origin or routing"),
                                location,
                            );
                            (format!("{meta:?}"), Vec::new())
                        }
                        _ => (format!("{meta:?}"), Vec::new()),
                    };
                    report.metadata.push(MetadataEvent {
                        track,
                        event,
                        tick,
                        kind,
                        data,
                    });
                    report.push(
                        "info",
                        "metadata_retained_in_report",
                        "source metadata retained separately from Score",
                        location,
                    );
                }
                other => report.push(
                    "error",
                    "unsupported_midi_event",
                    format!("{other:?}: SysEx/escape interpretation unsupported"),
                    location,
                ),
            }
        }
        for ((ch, key), queue) in pending {
            for (start, event, _) in queue {
                report.push(
                    "error",
                    "missing_note_off",
                    format!(
                        "channel {} key {key}: onset has no Note Off; no artificial end inserted",
                        ch + 1
                    ),
                    Some((track, event, start)),
                );
            }
        }
        names.push(name.unwrap_or_else(|| format!("Track {}", track + 1)));
    }
    report
        .source_notes
        .sort_by_key(|n| (n.track, n.start_tick, n.on_event));
    report.note_count = report.source_notes.len();
    if report.note_count != 0 {
        report.push("warning", "velocity_omitted", "source onset velocity retained in report; Score has no velocity or release-velocity field", None);
    }
    Ok((names, report))
}

pub fn import(bytes: &[u8], options: &ImportOptions) -> Result<(Score, Report), Failure> {
    let (names, mut report) = scan(bytes)?;
    let (timing, timeline) = prepare_timing(&mut report, options.reference_tempo)?;
    for n in report.source_notes.clone() {
        if !SKY_15_KEYS.contains(&n.pitch.as_str()) {
            report.push(
                "error",
                "unsupported_pitch",
                format!(
                    "MIDI {} ({}) is not in sky-15; no transposition/deletion applied",
                    n.key, n.pitch
                ),
                Some((n.track, n.on_event, n.start_tick)),
            );
        }
    }
    if report.failed() {
        return Err(report.failure());
    }
    let mut tracks: Vec<_> = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| Track {
            id: format!("midi-track-{}", i + 1),
            name,
            role: TrackRole::Unknown,
            notes: Vec::new(),
        })
        .collect();
    for (i, n) in report.source_notes.clone().into_iter().enumerate() {
        let loc = Some((n.track, n.on_event, n.start_tick));
        match (timing.endpoint(n.start_tick), timing.endpoint(n.end_tick)) {
            (Ok((start, a)), Ok((end, b))) => {
                if a || b {
                    report.push("warning", "time_quantized", format!("source endpoints {}..{} mapped to {start}..{end}; endpoints rounded independently, half up", n.start_tick, n.end_tick), loc);
                }
                if end <= start {
                    report.push(
                        "error",
                        "quantized_zero_duration",
                        "end minus start is not positive; note not deleted or extended",
                        loc,
                    );
                    continue;
                }
                tracks[n.track].notes.push(Note {
                    id: format!("midi-note-{}", i + 1),
                    pitch: Pitch::parse(n.pitch).expect("canonical MIDI pitch"),
                    start_tick: start,
                    duration_ticks: end - start,
                });
            }
            (Err(e), _) | (_, Err(e)) => report.push("error", "output_tick_range", e, loc),
        }
    }
    if report.failed() {
        return Err(report.failure());
    }
    let score = Score {
        format_version: SCORE_FORMAT_VERSION.into(),
        metadata: Metadata {
            id: "midi-import".into(),
            name: options
                .name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "MIDI import".into()),
        },
        instrument: Instrument {
            id: "sky-15".into(),
            name: "Sky 15-key instrument".into(),
            playback_mode: PlaybackMode::Trigger,
            keys: SKY_15_KEYS
                .iter()
                .map(|s| Pitch::parse(*s).unwrap())
                .collect(),
        },
        timeline,
        tracks,
        text: None,
    };
    if let Err(e) = score.validate() {
        report.push("error", "score_validation", e.to_string(), None);
        return Err(report.failure());
    }
    Ok((score, report))
}

pub(crate) fn prepare_timing(
    report: &mut Report,
    reference_tempo: Option<f64>,
) -> Result<(Timing, Timeline), Failure> {
    let mut tempos = BTreeMap::new();
    for t in report.tempos.clone() {
        let loc = Some((t.track, t.event, t.tick));
        if let Some((value, track, event)) = tempos.get(&t.tick) {
            if *value != t.microseconds_per_quarter {
                report.push(
                    "error",
                    "tempo_conflict",
                    format!(
                        "{} us/qn conflicts with {value} at track {track} event {event}",
                        t.microseconds_per_quarter
                    ),
                    loc,
                );
            } else {
                report.push(
                    "info",
                    "tempo_merged",
                    "identical same-tick tempo merged",
                    loc,
                );
            }
        } else {
            tempos.insert(t.tick, (t.microseconds_per_quarter, t.track, t.event));
        }
    }
    if let std::collections::btree_map::Entry::Vacant(entry) = tempos.entry(0) {
        entry.insert((500000, 0, 0));
        report.push(
            "warning",
            "default_tempo",
            "500000 us/qn applies from tick zero until the first explicit tempo",
            None,
        );
    }
    let bpm = reference_tempo.unwrap_or(60_000_000.0 / f64::from(tempos[&0].0));
    if !bpm.is_finite() || bpm <= 0.0 {
        report.push(
            "error",
            "invalid_reference_tempo",
            "target tempo must be finite and positive",
            None,
        );
    } else {
        report.reference_tempo = Some(bpm);
    }
    let mut meters = BTreeMap::new();
    for m in report.meters.clone() {
        let value = (
            m.numerator,
            m.denominator_power,
            m.clocks_per_click,
            m.thirty_seconds_per_quarter,
        );
        let loc = Some((m.track, m.event, m.tick));
        if let Some((previous, track, event)) = meters.get(&m.tick) {
            if *previous != value {
                report.push("error", "meter_conflict", format!("signature {value:?} conflicts with {previous:?} at track {track} event {event}"), loc);
            } else {
                report.push(
                    "info",
                    "meter_merged",
                    "identical same-tick signature merged",
                    loc,
                );
            }
        } else {
            meters.insert(m.tick, (value, m.track, m.event));
        }
        if m.tick != 0 {
            report.push(
                "warning",
                "meter_change_discarded",
                "later signature retained in report; nominal meter and note timing unchanged",
                loc,
            );
        }
    }
    let nominal = meters.get(&0).map(|m| m.0).unwrap_or_else(|| {
        report.push(
            "warning",
            "default_meter",
            "no initial signature; nominal meter is 4/4",
            None,
        );
        (4, 2, 24, 8)
    });
    if nominal.1 > 6 {
        report.push(
            "error",
            "nominal_meter_range",
            "initial denominator exceeds Score v1 limit 64",
            None,
        );
    }
    if report.failed() {
        return Err(report.clone().failure());
    }
    let map: Vec<_> = tempos
        .iter()
        .map(|(tick, (tempo, _, _))| (*tick, *tempo))
        .collect();
    if map.len() > 1 {
        report.push("warning", "tempo_map_flattened", "absolute elapsed timing mapped to reference tempo; original beat/bar structure is not preserved", None);
    }
    let timing = match Timing::new(report.source_ppq.unwrap(), &map, bpm) {
        Ok(timing) => timing,
        Err(e) => {
            report.push("error", "timing_calculation", e, None);
            return Err(report.clone().failure());
        }
    };
    Ok((
        timing,
        Timeline {
            ppq: PPQ,
            tempo: bpm,
            time_signature: TimeSignature {
                numerator: u16::from(nominal.0),
                denominator: 1_u16 << nominal.1,
            },
        },
    ))
}
