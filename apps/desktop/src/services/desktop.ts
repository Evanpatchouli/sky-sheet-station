import { invoke, isTauri } from "@tauri-apps/api/core";
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
export const selectAudio = () => desktopInvoke<string | null>("select_audio");
