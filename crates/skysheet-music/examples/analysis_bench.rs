//! Deterministic local measurement, not a representative music-quality corpus.
use skysheet_music::*;
use std::{hint::black_box, time::Instant};

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .map(|n| n.parse().expect("note count"))
        .unwrap_or(10_000);
    assert!((1..=100_000).contains(&count));
    let doc = MusicDocument {
        id: "benchmark-owned-synthetic".into(),
        source_name: None,
        source_format: "synthetic".into(),
        ppq: 480,
        tracks: (0..4)
            .map(|index| MusicTrack { index, name: None })
            .collect(),
        notes: (0..count)
            .map(|i| MusicNote {
                id: format!("n-{i}"),
                start_tick: (i / 4) as u64 * 480,
                end_tick: (i / 4) as u64 * 480 + 440,
                pitch: MidiPitch::new(36 + (i % 60) as u8).unwrap(),
                track: i % 4,
                midi: None,
                diagnostics: Vec::new(),
            })
            .collect(),
        tempo_map: Vec::new(),
        time_signature_map: Vec::new(),
        metadata: Vec::new(),
        annotations: Vec::new(),
        analysis_refs: Vec::new(),
        diagnostics: Vec::new(),
        original_midi: None,
    };
    let mut times = Vec::new();
    let mut json_bytes = 0;
    for _ in 0..20 {
        let start = Instant::now();
        let report = analyze(black_box(&doc)).unwrap();
        times.push(start.elapsed().as_micros());
        // Serialization is excluded from analysis time; included in process peak memory.
        json_bytes = serde_json::to_vec(black_box(&report)).unwrap().len();
        black_box(report);
    }
    times.sort();
    println!(
        "notes={count} iterations=20 min_us={} median_us={} max_us={} compact_json_bytes={json_bytes} serialization_in_timing=false",
        times[0], times[10], times[19]
    );
}
