//! Deterministic large synthetic workload derived by repeating the local MIDI.
fn main() {
    let mut doc = skysheet_midi::read_music(
        include_bytes!("../../../audios/ode-to-joy.mid"),
        Some("synthetic repeated ode".into()),
    )
    .unwrap();
    let original = std::mem::take(&mut doc.notes);
    doc.original_midi = None;
    for repetition in 0..200u64 {
        for n in &original {
            let mut note = n.clone();
            note.id = format!("{}-{repetition}", n.id);
            note.start_tick += repetition * 20000;
            note.end_tick += repetition * 20000;
            note.midi = None;
            doc.notes.push(note);
        }
    }
    let start = std::time::Instant::now();
    let projection = skysheet_midi::project_music(&doc).unwrap();
    let result = skysheet_arrange::arrange_with_options(
        &doc,
        &projection,
        skysheet_arrange::TransposeMode::Auto,
        &Default::default(),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&result.report).unwrap();
    println!(
        "notes={} kept={} elapsed_ms={} report_bytes={}",
        doc.notes.len(),
        result.report.kept,
        start.elapsed().as_millis(),
        bytes.len()
    );
}
