/** Display types only. All musical values and classifications come from the CLI. */
export type ExactInteger = number | string;
export interface MusicDiagnostic { category: string; code: string; message: string }
export interface ScoreComponent { name: string; raw: number; weight: number; contribution: number }
export interface MusicReport {
  report_version: 2;
  source_document_id: string;
  source: { source_name: string | null; ppq: number; time_signature_map: Array<{ location: { track: number; event: number; tick: ExactInteger }; numerator: number; denominator_power: number; clocks_per_click: number; thirty_seconds_per_quarter: number }> };
  note_statistics: { total_notes: number; max_polyphony: number; overlapping_note_pairs: number; onsets_per_quarter: number | null };
  rhythm_analysis: { unique_onsets: number; stability: string; modal_interval_fraction: number | null };
  key_candidates: { status: string };
  structure_analysis: {
    register_policy: string;
    low_register_end: number; high_register_start: number;
    registers: { low: number; middle: number; high: number };
    tracks: Array<{ track: number; name: string | null; note_count: number; active_ticks: ExactInteger; onsets_per_quarter: number | null; pitch_range: { lowest_name: string; highest_name: string; span_semitones: number } | null }>;
    track_pairs: Array<{ tracks: [number, number]; simultaneous_onsets: number; overlap_ticks: ExactInteger; onset_jaccard: number | null }>;
    pairwise_complete: boolean;
    meter_segments: Array<{ start_tick: ExactInteger; end_tick: ExactInteger | null; numerator: number; denominator_power: number; origin: string; bar_at_start: string | null }>;
    position_policy: string;
  };
  tempo_analysis: { effective_map: Array<{ start_tick: ExactInteger; microseconds_per_quarter: number; origin: string }>; change_count: number; principal_bpm: number | null };
  track_ranking: Array<{ rank: number; track: number; score: number; best_candidate_id: string | null; scoring: { eligible: boolean; components: ScoreComponent[]; weighted_sum: number; total: number } | null }>;
  track_roles: Array<{ track: number; role: string; candidates: Array<{ role: string; score: number; reasons: string[] }> }>;
  melody_candidates: Array<{ id: string; reasons: string[]; possible_errors: string[] }>;
  diagnostics: MusicDiagnostic[];
}
export interface LoadedAnalysis { source: string; report: MusicReport }

export function analysisErrorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return String(error);
}
