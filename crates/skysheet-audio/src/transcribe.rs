use crate::{
    Error, Note, Observer, Options, Report, SAMPLE_RATE, Transcription, decode, midi, progress,
};
use basic_pitch::{
    NoteParams, TrimMode,
    model::{Backend, TractBackend},
    notes::model_output_to_notes,
};
use std::{
    num::NonZeroUsize,
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

struct ObservedBackend<'a> {
    inner: TractBackend,
    observer: &'a Observer<'a>,
    completed: AtomicU64,
    total: u64,
    cancelled: AtomicBool,
}
impl Backend for ObservedBackend<'_> {
    fn predict_window(&self, window: &[f32]) -> basic_pitch::Result<basic_pitch::Posteriorgrams> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(basic_pitch::Error::backend(
                "cancelled",
                std::io::Error::other("cancelled"),
            ));
        }
        let output = self.inner.predict_window(window)?;
        let completed = self.completed.fetch_add(1, Ordering::Relaxed) + 1;
        if progress(self.observer, "transcribe", completed, Some(self.total)).is_err() {
            self.cancelled.store(true, Ordering::Relaxed);
            return Err(basic_pitch::Error::backend(
                "cancelled",
                std::io::Error::other("cancelled"),
            ));
        }
        Ok(output)
    }
}

pub fn transcribe(
    path: &Path,
    options: &Options,
    observer: &Observer<'_>,
) -> Result<Transcription, Error> {
    options.validate()?;
    let audio = decode(path, observer)?;
    progress(observer, "model", 0, None)?;
    let backend = ObservedBackend {
        inner: TractBackend::new().map_err(|e| Error::new("audio_model", e))?,
        observer,
        completed: AtomicU64::new(0),
        total: basic_pitch::window::window_count(audio.samples.len()) as u64,
        cancelled: AtomicBool::new(false),
    };
    progress(observer, "transcribe", 0, Some(backend.total))?;
    let workers = std::thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(4);
    let predictions = basic_pitch::model::run_inference_parallel(
        &backend,
        &audio.samples,
        TrimMode::Main,
        NonZeroUsize::new(workers).unwrap(),
    )
    .map_err(|e| {
        Error::new(
            if backend.cancelled.load(Ordering::Relaxed) {
                "cancelled"
            } else {
                "audio_inference"
            },
            e,
        )
    })?;
    progress(observer, "notes", 0, None)?;
    let params = NoteParams {
        onset_threshold: options.onset_threshold,
        frame_threshold: options.frame_threshold,
        minimum_note_length_ms: options.minimum_note_ms,
        include_pitch_bends: false,
        ..NoteParams::default()
    };
    let predicted_notes =
        model_output_to_notes(&predictions, &params).map_err(|e| Error::new("audio_notes", e))?;
    let mut diagnostics = vec![
        "Candidate transcription: mixed instruments, percussion, reverb and vocals can cause missed notes, false notes and octave errors. Review before use.".into(),
        "120 BPM and 4/4 are reference coordinates, not detected tempo/meter; absolute audio time and leading silence are retained.".into(),
        "Velocity is estimated from model note activation (1..127), not calibrated instrument force. Pitch bends and instrument identities are not inferred.".into(),
    ];
    if audio.source_channels > 1 {
        diagnostics
            .push("Channels were averaged to mono; opposite-phase stereo may cancel.".into());
    }
    let mut notes = Vec::with_capacity(predicted_notes.len());
    let mut clipped = 0;
    for note in predicted_notes {
        if !note.start_s.is_finite() || !note.end_s.is_finite() || !note.amplitude.is_finite() {
            return Err(Error::new(
                "audio_notes",
                "Model produced nonfinite note data",
            ));
        }
        let start = (note.start_s.max(0.0) * f64::from(SAMPLE_RATE)).round() as u64;
        let end = ((note.end_s.max(0.0) * f64::from(SAMPLE_RATE)).round() as u64)
            .min(audio.samples.len() as u64);
        if note.start_s < 0.0 || note.end_s * f64::from(SAMPLE_RATE) > audio.samples.len() as f64 {
            clipped += 1;
        }
        let start_tick = midi::sample_tick(start);
        let end_tick = midi::sample_tick(end);
        if end_tick <= start_tick {
            clipped += 1;
            continue;
        }
        notes.push(Note {
            pitch: note.pitch,
            start_sample: start,
            end_sample: end,
            start_tick,
            end_tick,
            velocity: note.velocity().clamp(1, 127),
            activation: f64::from(note.amplitude),
        });
    }
    if notes.len() > 100_000 {
        return Err(Error::new(
            "audio_note_limit",
            "Transcription exceeds 100000 notes",
        ));
    }
    // An onset retriggers a pitch. Clip a preceding same-pitch sustain so MIDI
    // note-on/off pairing remains unambiguous; preserve the intervention in report.
    notes.sort_by_key(|n| (n.pitch, n.start_tick, n.end_tick));
    let mut retriggers = 0;
    for index in 1..notes.len() {
        if notes[index - 1].pitch == notes[index].pitch
            && notes[index - 1].end_tick > notes[index].start_tick
        {
            notes[index - 1].end_tick = notes[index].start_tick;
            notes[index - 1].end_sample = notes[index].start_sample;
            retriggers += 1;
        }
    }
    notes.retain(|n| n.end_tick > n.start_tick);
    notes.sort_by_key(|n| (n.start_tick, n.pitch, n.end_tick));
    if clipped > 0 {
        diagnostics.push(format!("Clipped/removed {clipped} model endpoints outside decoded audio or below MIDI resolution."));
    }
    if retriggers > 0 {
        diagnostics.push(format!(
            "Shortened/removed {retriggers} overlapping same-pitch events at retriggers."
        ));
    }
    if notes.is_empty() {
        diagnostics.push("No pitched notes detected; the MIDI is empty. Try a clearer recording or lower thresholds.".into());
    }
    progress(observer, "midi", 0, Some(notes.len() as u64))?;
    let midi = midi::encode(&notes)?;
    let report = Report {
        report_version: 1,
        engine: "basic-pitch-rs-0.1.0/tract; Spotify nmp v0.4.0; main-tail-trim",
        model_sha256: basic_pitch::model::MODEL_SHA256,
        source_sample_rate: audio.source_sample_rate,
        source_channels: audio.source_channels,
        sample_rate: SAMPLE_RATE,
        sample_count: audio.samples.len(),
        duration_seconds: audio.samples.len() as f64 / f64::from(SAMPLE_RATE),
        reference_tempo: midi::TEMPO,
        ppq: midi::PPQ,
        options: options.clone(),
        notes,
        diagnostics,
    };
    progress(observer, "complete", 1, Some(1))?;
    Ok(Transcription { midi, report })
}
