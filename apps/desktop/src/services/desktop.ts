import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import type { AudioOptions, AudioProgress, LoadedAudio } from "../audio/model.ts";
import type { LoadedScore } from "../player/model.ts";
import type { LoadedAnalysis } from "../analysis/model.ts";
import type { LoadedArrangement, ArrangementOptions, ArrangementReport } from "../arrangement/model.ts";

export const desktopAvailable = () => isTauri();

function desktopInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktopAvailable())
    return Promise.reject(
      new Error("请在 SkySheet 桌面窗口打开文件；浏览器预览不提供 CLI 验证。"),
    );
  return invoke<T>(command, args);
}

export const openScore = () => desktopInvoke<LoadedScore | null>("open_score");
export const openMidi = () => desktopInvoke<LoadedScore | null>("open_midi");
export const analyzeMidi = () => desktopInvoke<LoadedAnalysis | null>("analyze_midi");
export const arrangeMidi = (sourceDocumentId: string, transpose: number | null, options: ArrangementOptions) => desktopInvoke<LoadedArrangement | null>("arrange_midi", {sourceDocumentId, transpose, options});
export const saveArrangement = (token: string) => desktopInvoke<ArrangementReport | null>("save_arrangement", {token});
export const cancelArrangement = () => desktopInvoke<void>("cancel_arrangement");
export const loadExample = () => desktopInvoke<LoadedScore>("load_example");
export const transcribeAudio = (requestId: string, options: AudioOptions, update: (value: AudioProgress) => void) => {
  const onProgress = new Channel<AudioProgress>();
  onProgress.onmessage = update;
  return desktopInvoke<LoadedAudio | null>("transcribe_audio", {requestId, options, onProgress});
};
export const cancelAudio = (requestId: string) => desktopInvoke<void>("cancel_audio", {requestId});
export const saveTranscription = (token: string) => desktopInvoke<string | null>("save_transcription", {token});
export const activateTranscription = (token: string) => desktopInvoke<void>("activate_transcription", {token});
