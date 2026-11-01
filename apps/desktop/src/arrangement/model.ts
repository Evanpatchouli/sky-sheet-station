import type { LoadedScore } from "../player/model.ts";

export interface Candidate {
  transpose_semitones: number;
  melody_total: number | null;
  melody_kept: number | null;
  melody_retention: number | null;
  total_kept: number;
  accompaniment_total: number;
  accompaniment_kept: number;
  accompaniment_retention: number | null;
  dropped: number;
  selected: boolean;
}

export interface NoteMapping {
  source_track: number;
  source_note_id: string;
  source_event_index: number | null;
  source_midi: number;
  target_midi: number;
  target_pitch: string | null;
  key_index: number | null;
  mapped: boolean;
  drop_reason: string | null;
  source_start_tick: number | string;
  source_duration_ticks: number | string;
  projected_start_tick: number;
  projected_duration_ticks: number;
}

export interface ArrangementReport {
  arrangement_report_version: 1;
  source_file: string | null;
  source_document_id: string;
  target_instrument: string;
  mode: string;
  transpose_semitones: number | null;
  original_notes: number;
  kept: number;
  dropped: number;
  retention: number | null;
  melody: { status: string; track: number | null; reason: string; confidence: string };
  candidates: Candidate[];
  tracks: { source_track: number; name: string | null; kept: number; dropped: number; retention: number | null; phase3_role: { role: string } }[];
  note_mappings: NoteMapping[];
  diagnostics: string[];
  score_generated: boolean;
  score_file: string | null;
  report_file: string | null;
  score_validation: boolean;
  time_preserved: boolean;
  pitch_mapping_valid: boolean;
}

export interface LoadedArrangement {
  loaded: LoadedScore;
  report: ArrangementReport;
}
