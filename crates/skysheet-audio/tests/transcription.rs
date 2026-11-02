use skysheet_audio::{Options, SAMPLE_RATE, decode, transcribe};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn bytes(bytes: &[u8], extension: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "skysheet-at-{}-{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            extension
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn wav(rate: u32, channels: u16, seconds: f64, sound: impl Fn(f64) -> f64) -> Self {
        let frames = (f64::from(rate) * seconds) as u32;
        let size = frames * u32::from(channels) * 2;
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend((36 + size).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(channels.to_le_bytes());
        bytes.extend(rate.to_le_bytes());
        bytes.extend((rate * u32::from(channels) * 2).to_le_bytes());
        bytes.extend((channels * 2).to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(size.to_le_bytes());
        for frame in 0..frames {
            let sample = (sound(f64::from(frame) / f64::from(rate)) * 32767.0) as i16;
            for _ in 0..channels {
                bytes.extend(sample.to_le_bytes());
            }
        }
        Self::bytes(&bytes, "wav")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn tone(t: f64, pitch: u8) -> f64 {
    let hz = 440.0 * 2.0f64.powf((f64::from(pitch) - 69.0) / 12.0);
    (std::f64::consts::TAU * hz * t).sin()
}

#[test]
fn decoder_preserves_duration_leading_silence_and_pitch_across_rates() {
    for rate in [16_000, 22_050, 44_100, 48_000] {
        let fixture = Fixture::wav(
            rate,
            2,
            1.0,
            |t| if t < 0.25 { 0.0 } else { 0.4 * tone(t, 69) },
        );
        let audio = decode(&fixture.0, &|_| true).unwrap();
        assert_eq!(audio.samples.len(), SAMPLE_RATE as usize);
        assert_eq!(audio.source_sample_rate, rate);
        assert_eq!(audio.source_channels, 2);
        assert!(audio.samples[..4000].iter().all(|v| v.abs() < 0.0001));
        let crossing = audio.samples[11025..]
            .windows(2)
            .filter(|p| p[0] <= 0.0 && p[1] > 0.0)
            .count();
        assert!((219..=221).contains(&crossing), "rate {rate}: {crossing}");
    }
}

#[test]
fn malformed_empty_missing_cancelled_and_invalid_options_fail() {
    for bytes in [b"not an mp3".as_slice(), b""] {
        let fixture = Fixture::bytes(bytes, "mp3");
        assert!(decode(&fixture.0, &|_| true).is_err());
    }
    let fixture = Fixture::wav(44_100, 1, 1.0, |_| 0.0);
    assert_eq!(
        decode(&fixture.0, &|_| false).err().unwrap().code,
        "cancelled"
    );
    assert_eq!(
        transcribe(
            &fixture.0,
            &Options {
                frame_threshold: f64::NAN,
                ..Options::default()
            },
            &|_| true
        )
        .err()
        .unwrap()
        .code,
        "invalid_options"
    );
    assert_eq!(
        decode(&fixture.0.with_extension("missing"), &|_| true)
            .err()
            .unwrap()
            .code,
        "audio_read"
    );
}

#[test]
fn real_model_recognizes_reference_notes_chord_times_and_midi_roundtrip() {
    let fixture = Fixture::wav(44_100, 2, 4.0, |t| {
        if (0.5..1.5).contains(&t) {
            0.45 * tone(t, 69)
        } else if (2.0..3.3).contains(&t) {
            0.22 * (tone(t, 72) + tone(t, 76))
        } else {
            0.0
        }
    });
    let stages = Mutex::new(Vec::new());
    let result = transcribe(&fixture.0, &Options::default(), &|p| {
        stages.lock().unwrap().push(p.stage);
        true
    })
    .unwrap();
    for (pitch, start, end) in [(69, 0.5, 1.5), (72, 2.0, 3.3), (76, 2.0, 3.3)] {
        let note = result
            .report
            .notes
            .iter()
            .filter(|n| n.pitch == pitch)
            .max_by_key(|n| n.end_tick - n.start_tick)
            .unwrap_or_else(|| panic!("missing {pitch}: {:?}", result.report.notes));
        assert!(
            (note.start_sample as f64 / f64::from(SAMPLE_RATE) - start).abs() < 0.15,
            "{note:?}"
        );
        assert!(
            (note.end_sample as f64 / f64::from(SAMPLE_RATE) - end).abs() < 0.25,
            "{note:?}"
        );
        assert!((1..=127).contains(&note.velocity));
    }
    let music = skysheet_midi::read_music(&result.midi, None).unwrap();
    assert_eq!(music.notes.len(), result.report.notes.len());
    let inspected = skysheet_midi::inspect(&result.midi).unwrap();
    for (actual, expected) in inspected.source_notes.iter().zip(&result.report.notes) {
        assert_eq!(
            (
                actual.key,
                actual.start_tick,
                actual.end_tick,
                actual.velocity
            ),
            (
                expected.pitch,
                expected.start_tick,
                expected.end_tick,
                expected.velocity
            )
        );
    }
    let stages = stages.lock().unwrap();
    assert_eq!(stages.first(), Some(&"decode"));
    assert_eq!(stages.last(), Some(&"complete"));
    assert!(stages.contains(&"transcribe"));
}

#[test]
fn silence_produces_empty_real_midi_and_explicit_diagnostic() {
    let fixture = Fixture::wav(22_050, 1, 0.5, |_| 0.0);
    let result = transcribe(&fixture.0, &Options::default(), &|_| true).unwrap();
    assert!(result.report.notes.is_empty());
    assert!(
        result
            .report
            .diagnostics
            .iter()
            .any(|s| s.contains("No pitched notes"))
    );
    assert_eq!(skysheet_midi::inspect(&result.midi).unwrap().note_count, 0);
}

#[test]
fn owner_mp3_decodes_without_external_tools() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../audios/Scarlxrd-Ace - 小星星(钢琴版).mp3");
    let audio = decode(&path, &|_| true).unwrap();
    assert!(audio.samples.len() > 60 * SAMPLE_RATE as usize);
    assert!(audio.samples.iter().any(|sample| sample.abs() > 0.1));
}

#[test]
fn truncated_pcm_is_not_silently_shortened() {
    let fixture = Fixture::wav(22_050, 1, 1.0, |_| 0.0);
    let mut bytes = std::fs::read(&fixture.0).unwrap();
    bytes.truncate(bytes.len() / 2);
    std::fs::write(&fixture.0, bytes).unwrap();
    assert!(decode(&fixture.0, &|_| true).is_err());
}

#[test]
fn cancellation_interrupts_real_inference_before_midi_is_returned() {
    let fixture = Fixture::wav(22_050, 1, 3.0, |t| 0.4 * tone(t, 69));
    let result = transcribe(&fixture.0, &Options::default(), &|p| {
        p.stage != "transcribe" || p.completed == 0
    });
    assert_eq!(result.err().unwrap().code, "cancelled");
}
