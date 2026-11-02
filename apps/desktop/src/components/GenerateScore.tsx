import { Select } from "./Select.tsx";
import { MusicProgress } from "./MusicProgress.tsx";
import { useEffect, useRef, useState } from "react";
import { Modal } from "./Modal.tsx";
import { Icon } from "./Icon.tsx";
import {
  defaultArrangementOptions,
  optionsError,
} from "../arrangement/model.ts";
import type {
  ArrangementOptions,
  LoadedArrangement,
} from "../arrangement/model.ts";
import {
  audioOptionsError,
  defaultAudioOptions,
  progressPercent,
} from "../audio/model.ts";
import type { AudioProgress, LoadedAudio } from "../audio/model.ts";
import type { LoadedAnalysis } from "../analysis/model.ts";
import { analysisErrorMessage } from "../analysis/model.ts";
import {
  analyzeMidi,
  arrangeMidi,
  cancelArrangement,
  cancelAudio,
  selectSource,
  transcribeAudio,
} from "../services/desktop.ts";
import type { SelectedSource } from "../services/desktop.ts";

export interface GenerationResult {
  arrangement: LoadedArrangement;
  analysis: LoadedAnalysis;
  audio: LoadedAudio | null;
}

export function GenerateScore({
  onClose,
  onGenerated,
}: {
  onClose: () => void;
  onGenerated: (result: GenerationResult) => void;
}) {
  const [source, setSource] = useState<SelectedSource | null>(null);
  const [options, setOptions] = useState(defaultArrangementOptions);
  const [audioOptions, setAudioOptions] = useState(defaultAudioOptions);
  const [shift, setShift] = useState("auto"),
    [transpose, setTranspose] = useState("0");
  const [working, setWorking] = useState(false),
    [picking, setPicking] = useState(false);
  const [error, setError] = useState<string | null>(null),
    [stage, setStage] = useState("");
  const [progress, setProgress] = useState<AudioProgress | null>(null);
  const request = useRef<string | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      const id = request.current;
      request.current = null;
      if (id) {
        void cancelAudio(id).catch(() => {});
        void cancelArrangement().catch(() => {});
      }
    };
  }, []);
  async function pick() {
    setPicking(true);
    setError(null);
    try {
      const chosen = await selectSource();
      if (chosen && alive.current) setSource(chosen);
    } catch (e) {
      if (alive.current) setError(analysisErrorMessage(e));
    } finally {
      if (alive.current) setPicking(false);
    }
  }
  async function generate() {
    if (!source || working) return;
    const invalid =
      optionsError(options) ||
      (source.kind === "audio" ? audioOptionsError(audioOptions) : null);
    if (invalid) {
      setError(invalid);
      return;
    }
    const semitones = shift === "auto" ? null : Number(transpose);
    if (
      semitones !== null &&
      (!transpose.trim() ||
        !Number.isInteger(semitones) ||
        Math.abs(semitones) > 127)
    ) {
      setError("移调必须是 -127 到 127 的整数。");
      return;
    }
    const id = crypto.randomUUID();
    request.current = id;
    setWorking(true);
    setError(null);
    setStage("正在读取音乐…");
    try {
      let audio: LoadedAudio | null = null;
      let analysis: LoadedAnalysis | null;
      if (source.kind === "audio") {
        audio = await transcribeAudio(
          id,
          audioOptions,
          (next) => {
            if (request.current !== id) return;
            setStage("正在识别音乐中的音符…");
            setProgress((previous) =>
              previous?.stage === next.stage &&
              previous.completed > next.completed
                ? previous
                : next,
            );
          },
          source.token,
        );
        analysis = audio?.analysis ?? null;
      } else analysis = await analyzeMidi(source.token);
      if (request.current !== id) return;
      if (!analysis) throw new Error("未读取到音乐，请重新选择文件。");
      if (audio && audio.report.notes.length === 0)
        throw new Error("没有识别到可用音符，请调整识别参数后重试。");
      setStage("正在生成适合 Sky 的乐谱…");
      setProgress(null);
      const arrangement = await arrangeMidi(
        analysis.report.source_document_id,
        semitones,
        options,
      );
      if (request.current !== id) return;
      if (!arrangement) throw new Error("未生成乐谱，请重试。");
      request.current = null;
      onGenerated({ arrangement, analysis, audio });
    } catch (e) {
      if (request.current === id) setError(analysisErrorMessage(e));
    } finally {
      if (request.current === id) {
        request.current = null;
        setWorking(false);
        setProgress(null);
        setStage("");
      }
    }
  }
  async function close() {
    if (picking) return;
    const id = request.current;
    request.current = null;
    if (id) {
      setStage("正在取消…");
      const results = await Promise.allSettled([
        cancelAudio(id),
        cancelArrangement(),
      ]);
      const failed = results.find((result) => result.status === "rejected");
      if (failed?.status === "rejected") {
        setWorking(false);
        setError(analysisErrorMessage(failed.reason));
        return;
      }
    }
    onClose();
  }
  const percent = progress ? progressPercent(progress) : undefined;
  return (
    <Modal title="导入音乐，生成乐谱" onClose={() => void close()}>
      <p className="muted">导入音频或 MIDI，生成后直接编辑和试听。</p>
      <div className="source-picker">
        <div>
          <strong>{source?.name ?? "选择一份音乐文件"}</strong>
          <span>
            {source
              ? source.kind === "audio"
                ? "音频 · MP3 / WAV / FLAC"
                : "MIDI"
              : "MIDI、MP3、WAV、FLAC"}
          </span>
        </div>
        <button disabled={working || picking} onClick={() => void pick()}>
          <Icon name="open" />
          {picking ? "选择中…" : source ? "更换文件" : "选择文件"}
        </button>
      </div>
      <div className="generation-mode">
        <label>
          编曲方式{" "}
          <Select
            aria-label="编曲方式"
            disabled={working}
            value={options.mode}
            onChange={(value) =>
              setOptions({
                ...options,
                mode: value as ArrangementOptions["mode"],
              })
            }
          >
            <option
              value="melody-first"
              data-description="突出主旋律，适合第一次尝试"
            >
              旋律优先
            </option>
            <option
              value="simple-accompaniment"
              data-description="保留主旋律与精简伴奏"
            >
              简易伴奏
            </option>
            <option
              value="polyphonic"
              data-description="尽量保留多个声部，让乐谱更丰富"
            >
              多声部优先
            </option>
          </Select>
        </label>
        <p className="muted">
          {options.mode === "melody-first"
            ? "突出主旋律，适合第一次尝试。"
            : options.mode === "simple-accompaniment"
              ? "保留主旋律与精简伴奏。"
              : "尽量保留多个声部，乐谱会更丰富。"}
        </p>
      </div>
      <details className="generation-advanced">
        <summary>高级设置</summary>
        <div className="arrangement-controls">
          <label>
            音域适配{" "}
            <Select
              aria-label="音域适配"
              value={shift}
              disabled={working}
              onChange={(value) => setShift(value)}
            >
              <option value="auto">自动</option>
              <option value="fixed">固定移调</option>
            </Select>
          </label>
          {shift === "fixed" && (
            <label>
              半音数
              <input
                aria-label="移调半音数"
                type="number"
                min={-127}
                max={127}
                step={1}
                value={transpose}
                disabled={working}
                onChange={(e) => setTranspose(e.target.value)}
              />
            </label>
          )}
          <label>
            <input
              type="checkbox"
              checked={options.allow_octave_shifts}
              disabled={working}
              onChange={(e) =>
                setOptions({
                  ...options,
                  allow_octave_shifts: e.target.checked,
                })
              }
            />
            允许八度调整
          </label>
          {(
            [
              ["melody_priority", "旋律优先级"],
              ["bass_priority", "低音优先级"],
              ["accompaniment_priority", "伴奏优先级"],
            ] as const
          ).map(([key, label]) => (
            <label key={key}>
              {label}
              <input
                aria-label={label}
                type="number"
                min={0}
                max={5}
                step={1}
                value={Number.isNaN(options[key]) ? "" : options[key]}
                disabled={working}
                onChange={(e) =>
                  setOptions({
                    ...options,
                    [key]: e.target.value === "" ? NaN : Number(e.target.value),
                  })
                }
              />
            </label>
          ))}
          <label>
            最大同时发声数
            <input
              aria-label="最大同时发声数"
              type="number"
              min={1}
              max={128}
              step={1}
              placeholder="不限"
              value={options.maximum_polyphony ?? ""}
              disabled={working}
              onChange={(e) =>
                setOptions({
                  ...options,
                  maximum_polyphony:
                    e.target.value === "" ? null : Number(e.target.value),
                })
              }
            />
          </label>
          {source?.kind === "audio" && (
            <>
              <label>
                起音阈值
                <input
                  aria-label="起音阈值"
                  type="number"
                  min={0.05}
                  max={0.95}
                  step={0.05}
                  value={
                    Number.isNaN(audioOptions.onset_threshold)
                      ? ""
                      : audioOptions.onset_threshold
                  }
                  disabled={working}
                  onChange={(e) =>
                    setAudioOptions({
                      ...audioOptions,
                      onset_threshold:
                        e.target.value === "" ? NaN : Number(e.target.value),
                    })
                  }
                />
              </label>
              <label>
                持续阈值
                <input
                  aria-label="持续阈值"
                  type="number"
                  min={0.05}
                  max={0.95}
                  step={0.05}
                  value={
                    Number.isNaN(audioOptions.frame_threshold)
                      ? ""
                      : audioOptions.frame_threshold
                  }
                  disabled={working}
                  onChange={(e) =>
                    setAudioOptions({
                      ...audioOptions,
                      frame_threshold:
                        e.target.value === "" ? NaN : Number(e.target.value),
                    })
                  }
                />
              </label>
              <label>
                最短音符 / ms
                <input
                  aria-label="最短音符"
                  type="number"
                  min={30}
                  max={1000}
                  step="any"
                  value={
                    Number.isNaN(audioOptions.minimum_note_ms)
                      ? ""
                      : audioOptions.minimum_note_ms
                  }
                  disabled={working}
                  onChange={(e) =>
                    setAudioOptions({
                      ...audioOptions,
                      minimum_note_ms:
                        e.target.value === "" ? NaN : Number(e.target.value),
                    })
                  }
                />
              </label>
            </>
          )}
        </div>
        {source?.kind === "audio" && (
          <p className="muted">
            音频最长 10 分钟、128 MiB。较低阈值可能找回弱音，也可能增加误识别。
          </p>
        )}
      </details>
      {working && <MusicProgress label={stage} value={percent} />}
      {error && (
        <p className="notice error" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button disabled={picking} onClick={() => void close()}>
          {working ? "取消生成" : "取消"}
        </button>
        <button
          className="primary"
          disabled={!source || working || picking}
          onClick={() => void generate()}
        >
          <Icon name="audio" />
          {working ? "正在生成…" : "生成乐谱"}
        </button>
      </div>
    </Modal>
  );
}
