//! Adapter only: source parsing remains in the Phase 2 scanner.
use crate::Failure;
use skysheet_music::{
    Diagnostic, EventLocation, MidiNoteSource, MidiPitch, MusicDocument, MusicNote, MusicTrack,
    SourceMetadata,
};
use std::collections::BTreeMap;

pub fn read_music(bytes: &[u8], source_name: Option<String>) -> Result<MusicDocument, Failure> {
    let mut report = crate::inspect(bytes)?;
    // Do not certify incomplete pairings or controller-dependent sounding notes.
    // Inspection itself retains its existing permissive diagnostic contract.
    if report.failed() {
        return Err(report.failure());
    }
    let location = |track, event, tick| EventLocation { track, event, tick };
    let diagnostics:Vec<_> = report.diagnostics.iter().map(|d| {
        let (code,message) = match d.code {
            "velocity_omitted" => ("velocity_retained","Onset velocity retained in MusicDocument; original MIDI bytes also retain release velocity.".into()),
            "program_omitted" => ("program_retained","Program event retained as metadata; it does not determine the inferred role.".into()),
            "meter_click_omitted" => ("meter_click_retained","Original meter parameters retained; nominal downbeats are an analytical convention.".into()),
            _ => (d.code,d.message.clone()),
        };
        Diagnostic {category:format!("source_{}",d.severity),code:code.into(),message,
            location:d.track.zip(d.event).zip(d.source_tick).map(|((t,e),s)|location(t,e,s))}
    }).collect();
    let mut event_diagnostics: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for diagnostic in &diagnostics {
        if let Some(location) = &diagnostic.location {
            event_diagnostics
                .entry((location.track, location.event))
                .or_default()
                .push(diagnostic);
        }
    }
    let mut track_names = BTreeMap::new();
    for metadata in &report.metadata {
        if metadata.kind == "track_name" && !metadata.data.is_empty() {
            track_names
                .entry(metadata.track)
                .or_insert_with(|| String::from_utf8_lossy(&metadata.data).into_owned());
        }
    }
    let fingerprint = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, b| {
        (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    let doc = MusicDocument {
        id: format!("midi-fnv1a64-{fingerprint:016x}-{}", bytes.len()),
        source_name,
        source_format: format!("smf-format-{}", report.format.unwrap()),
        ppq: report.source_ppq.unwrap(),
        tracks: (0..report.track_count)
            .map(|index| MusicTrack {
                index,
                name: track_names.remove(&index),
            })
            .collect(),
        notes: report
            .source_notes
            .iter()
            .map(|n| MusicNote {
                id: format!("track-{}-on-{}", n.track, n.on_event),
                start_tick: n.start_tick,
                end_tick: n.end_tick,
                pitch: MidiPitch::new(n.key).expect("validated MIDI data byte"),
                track: n.track,
                midi: Some(MidiNoteSource {
                    velocity: n.velocity,
                    channel: n.channel,
                    on_event: n.on_event,
                    off_event: n.off_event,
                }),
                diagnostics: [n.on_event, n.off_event]
                    .iter()
                    .filter_map(|event| event_diagnostics.get(&(n.track, *event)))
                    .flatten()
                    .map(|d| (*d).clone())
                    .collect(),
            })
            .collect(),
        tempo_map: report
            .tempos
            .iter()
            .map(|t| skysheet_music::TempoEvent {
                location: location(t.track, t.event, t.tick),
                microseconds_per_quarter: t.microseconds_per_quarter,
            })
            .collect(),
        time_signature_map: report
            .meters
            .iter()
            .map(|m| skysheet_music::MeterEvent {
                location: location(m.track, m.event, m.tick),
                numerator: m.numerator,
                denominator_power: m.denominator_power,
                clocks_per_click: m.clocks_per_click,
                thirty_seconds_per_quarter: m.thirty_seconds_per_quarter,
            })
            .collect(),
        metadata: report
            .metadata
            .iter()
            .map(|m| SourceMetadata {
                location: location(m.track, m.event, m.tick),
                kind: m.kind.clone(),
                data: m.data.clone(),
            })
            .collect(),
        annotations: Vec::new(),
        analysis_refs: Vec::new(),
        diagnostics,
        original_midi: Some(bytes.to_vec()),
    };
    if let Err(error) = doc.validate() {
        report.push("error", "music_document", error.to_string(), None);
        return Err(report.failure());
    }
    Ok(doc)
}
