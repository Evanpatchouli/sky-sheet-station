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
  global_transposed_midi: number;
  voice_octave_shift: number;
  mapping_reason: string;
  previous_interval_change: number | null;
  next_interval_change: number | null;
  importance: number;
  importance_reasons: string[];
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
  arrangement_report_version: 2;
  arrangement_mode: string;
  role_retention: {role: string; kept: number; total: number; retention: number | null}[];
  playability: { maximum_simultaneous_notes: number; octave_adjusted_notes: number; melody_interval_changes: number; unnecessary_large_leaps: number; same_key_overlap_events: number; cross_track_key_conflicts: number; bass_melody_crossing_events: number; source_accompaniment_onsets: number; retained_accompaniment_onsets: number; arrangement_constraints_satisfied: boolean; human_audition: string };
  onset_groups: {start_tick: number; source_note_ids: string[]; kept: number; chord_candidate: string | null; exact_intervals_changed: boolean}[];
  conflicts: {start_tick: number; source_note_ids: string[]; source_count: number; kept: number; reason: string; resolution: string}[];
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
  tracks: { source_track: number; name: string | null; kept: number; dropped: number; retention: number | null; phase3_role: { role: string }; source_lowest_midi: number | null; source_highest_midi: number | null; final_lowest_midi: number | null; final_highest_midi: number | null; adjusted: number }[];
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
  token: string;
  loaded: LoadedScore;
  report: ArrangementReport;
}

export interface ArrangementOptions {
  mode: "melody-first" | "simple-accompaniment" | "polyphonic";
  allow_octave_shifts: boolean;
  melody_priority: number;
  bass_priority: number;
  accompaniment_priority: number;
  maximum_polyphony: number | null;
}
export const defaultArrangementOptions: ArrangementOptions = {mode:"melody-first",allow_octave_shifts:true,melody_priority:3,bass_priority:2,accompaniment_priority:1,maximum_polyphony:null};
export function optionsError(o: ArrangementOptions): string | null {
  if (![o.melody_priority,o.bass_priority,o.accompaniment_priority].every(v=>Number.isInteger(v)&&v>=0&&v<=5)) return "优先级必须是 0–5 的整数。";
  if(o.maximum_polyphony!==null&&(!Number.isInteger(o.maximum_polyphony)||o.maximum_polyphony<1||o.maximum_polyphony>128)) return "最大同时发声数必须是 1–128 的整数，留空表示不限。";
  return null;
}
