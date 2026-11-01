use crate::{Error, Note, SAMPLE_RATE};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

pub const PPQ: u16 = 960;
pub const TEMPO: u16 = 120;

/// Absolute audio-sample endpoints -> 120 BPM, 960 PPQ. Exact integer half-up
/// rounding; this is audio's time grid, not a replacement for MIDI's D6 engine.
pub fn sample_tick(sample: u64) -> u64 {
    (sample * u64::from(PPQ) * 2 + u64::from(SAMPLE_RATE) / 2) / u64::from(SAMPLE_RATE)
}

pub fn encode(notes: &[Note]) -> Result<Vec<u8>, Error> {
    let mut events = Vec::with_capacity(notes.len() * 2);
    for note in notes {
        if note.pitch > 127
            || !(1..=127).contains(&note.velocity)
            || note.end_tick <= note.start_tick
        {
            return Err(Error::new("audio_midi", "Invalid transcribed MIDI note"));
        }
        events.push((note.start_tick, 1u8, note.pitch, note.velocity));
        events.push((note.end_tick, 0u8, note.pitch, 0));
    }
    // Release before retriggering the same pitch at an equal tick.
    events.sort_unstable();
    let mut track = vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(b"Audio transcription")),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(500_000.into())),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
    ];
    let mut previous = 0;
    for (tick, on, pitch, velocity) in events {
        let delta = tick - previous;
        if delta > 0x0fff_ffff {
            return Err(Error::new("audio_midi", "MIDI delta exceeds SMF range"));
        }
        track.push(TrackEvent {
            delta: (delta as u32).into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: if on == 1 {
                    MidiMessage::NoteOn {
                        key: pitch.into(),
                        vel: velocity.into(),
                    }
                } else {
                    MidiMessage::NoteOff {
                        key: pitch.into(),
                        vel: 0.into(),
                    }
                },
            },
        });
        previous = tick;
    }
    track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(PPQ.into())),
        tracks: vec![track],
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .map_err(|e| Error::new("audio_midi", e))?;
    Ok(bytes)
}
