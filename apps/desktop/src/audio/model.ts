import type { LoadedAnalysis } from "../analysis/model.ts";

export interface AudioOptions { onset_threshold: number; frame_threshold: number; minimum_note_ms: number }
export const defaultAudioOptions: AudioOptions = { onset_threshold: 0.5, frame_threshold: 0.3, minimum_note_ms: 127.7 };
export interface AudioProgress { stage: string; completed: number; total: number | null }
export interface TranscribedNote { pitch: number; start_sample: number; end_sample: number; start_tick: number; end_tick: number; velocity: number; activation: number }
export interface AudioReport {
  report_version: number; engine: string; duration_seconds: number; source_sample_rate: number;
  source_channels: number; sample_rate: number; notes: TranscribedNote[]; diagnostics: string[];
}
export interface LoadedAudio { token: string; source: string; report: AudioReport; analysis: LoadedAnalysis }
export const stageLabels: Record<string, string> = {
  pick: "选择音频", import: "导入音频", decode: "解码音频", resample: "准备音频",
  model: "加载本地模型", transcribe: "识别音高与音符", notes: "整理音符边界", midi: "生成 MIDI",
  complete: "MIDI 已生成", analyze: "分析转录 MIDI",
};
export function progressPercent(progress: AudioProgress): number | undefined {
  if (!progress.total || progress.total < 0 || !Number.isFinite(progress.total) || !Number.isFinite(progress.completed)) return undefined;
  return Math.max(0, Math.min(100, Math.round(progress.completed / progress.total * 100)));
}
export function audioOptionsError(options: AudioOptions): string | null {
  if ([options.onset_threshold, options.frame_threshold].some(v => !Number.isFinite(v) || v < 0.05 || v > 0.95)) return "识别阈值须在 0.05–0.95 之间。";
  if (!Number.isFinite(options.minimum_note_ms) || options.minimum_note_ms < 30 || options.minimum_note_ms > 1000) return "最短音符须在 30–1000 毫秒之间。";
  return null;
}
