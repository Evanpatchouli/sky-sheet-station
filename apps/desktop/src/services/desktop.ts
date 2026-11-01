import { invoke, isTauri } from "@tauri-apps/api/core";
import type { LoadedScore } from "../player/model.ts";

export const desktopAvailable = () => isTauri();

function desktopInvoke<T>(command: string): Promise<T> {
  if (!desktopAvailable())
    return Promise.reject(
      new Error("请在 SkySheet 桌面窗口打开文件；浏览器预览不提供 CLI 验证。"),
    );
  return invoke<T>(command);
}

export const openScore = () => desktopInvoke<LoadedScore | null>("open_score");
export const loadExample = () => desktopInvoke<LoadedScore>("load_example");
export const selectAudio = () => desktopInvoke<string | null>("select_audio");
