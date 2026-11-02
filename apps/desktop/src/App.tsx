import { useEffect, useState, useSyncExternalStore } from "react";
import { Icon } from "./components/Icon.tsx";
import { ScoreEditor } from "./components/ScoreEditor.tsx";
import { GenerateScore } from "./components/GenerateScore.tsx";
import type { GenerationResult } from "./components/GenerateScore.tsx";
import { Transport } from "./components/Transport.tsx";
import { Modal } from "./components/Modal.tsx";
import { MusicAnalysis } from "./components/MusicAnalysis.tsx";
import { ArrangementReport } from "./components/ArrangementReport.tsx";
import { PlayerEngine } from "./player/engine.ts";
import { prepareScore } from "./player/model.ts";
import type { LoadedScore, PreparedScore, Score } from "./player/model.ts";
import { analysisErrorMessage } from "./analysis/model.ts";
import {
  loadExample,
  openScore,
  saveScore,
  saveArrangement,
  saveTranscription,
} from "./services/desktop.ts";

export default function App() {
  const [engine] = useState(() => new PlayerEngine());
  const state = useSyncExternalStore(engine.subscribe, engine.snapshot);
  const [song, setSong] = useState<PreparedScore | null>(null);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<
    "info" | "settings" | "generate" | "details" | null
  >(null);
  const [generation, setGeneration] = useState<GenerationResult | null>(null);
  const [dirty, setDirty] = useState(false),
    [edited, setEdited] = useState(false);
  const [session, setSession] = useState(0),
    [volume, setVolume] = useState(0.65);
  const [notice, setNotice] = useState("");
  const [transportPosition, setTransportPosition] = useState<{
    seconds: number;
  } | null>(null);
  const [pendingAction, setPendingAction] = useState<(() => void) | null>(null);
  useEffect(() => () => engine.dispose(), [engine]);
  useEffect(() => {
    const protect = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", protect);
    return () => window.removeEventListener("beforeunload", protect);
  }, [dirty]);
  function install(
    loaded: LoadedScore,
    generated: GenerationResult | null = null,
  ) {
    const prepared = prepareScore(loaded);
    engine.load(prepared);
    setSong(prepared);
    setGeneration(generated);
    setTransportPosition(null);
    setSession((value) => value + 1);
    setDirty(!!generated);
    setEdited(false);
    setNotice("");
  }
  async function load(action: () => Promise<LoadedScore | null>) {
    engine.pause();
    setBusy(true);
    setError(null);
    try {
      const loaded = await action();
      if (loaded) install(loaded);
    } catch (e) {
      setError(analysisErrorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  function replace(action: () => void) {
    engine.pause();
    if (dirty) setPendingAction(() => action);
    else action();
  }
  async function save() {
    if (!song || busy) return false;
    setBusy(true);
    setError(null);
    try {
      const path = await saveScore(song.score);
      if (!path) return false;
      setDirty(false);
      setNotice(`已保存：${path}`);
      return true;
    } catch (e) {
      setError(analysisErrorMessage(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function saveReport(kind: "arrangement" | "audio") {
    if (!generation || busy) return;
    setBusy(true);
    setError(null);
    try {
      if (kind === "arrangement") {
        const result = await saveArrangement(generation.arrangement.token);
        if (result) {
          setDirty(false);
          setNotice(`已保存原始生成乐谱及报告：${result.score_file}`);
        }
      } else if (generation.audio) {
        const path = await saveTranscription(generation.audio.token);
        if (path) setNotice(`已保存转录 MIDI 及报告：${path}`);
      }
    } catch (e) {
      setError(analysisErrorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  function edit(score: Score) {
    if (!song) return;
    engine.pause();
    const position = engine.snapshot().position;
    const prepared = prepareScore({ ...song, score });
    engine.load(prepared);
    engine.seek(position);
    setSong(prepared);
    setDirty(true);
    setEdited(true);
    setNotice("");
  }
  const message = error || state.error;
  return (
    <main className="app">
      <header className="toolbar">
        <div className="brand">
          Sky<span>Sheet</span>
          <i />
        </div>
        <div className="score-title">
          <strong>{song?.score.metadata.name ?? "乐谱工作站"}</strong>
          <span>
            {song
              ? `${song.score.tracks.length} 轨 · ${song.notes.length} 音符${dirty ? " · 未保存" : ""}`
              : "导入音乐，开始编曲"}
          </span>
        </div>
        <nav aria-label="文件与设置">
          <button
            className="primary"
            disabled={busy}
            onClick={() =>
              replace(() => {
                setError(null);
                setModal("generate");
              })
            }
          >
            <Icon name="audio" />
            <span>导入音乐</span>
          </button>
          <button
            disabled={busy}
            onClick={() => replace(() => void load(openScore))}
          >
            <Icon name="open" />
            <span>打开乐谱</span>
          </button>
          <button
            aria-label="另存乐谱"
            disabled={!song || busy}
            onClick={() => void save()}
          >
            <Icon name="save" />
            <span>另存乐谱</span>
          </button>
          <button
            aria-label="乐谱信息"
            disabled={!song}
            onClick={() => setModal("info")}
          >
            <Icon name="info" />
            <span>乐谱信息</span>
          </button>
          <button aria-label="设置" onClick={() => setModal("settings")}>
            <Icon name="settings" />
            <span>设置</span>
          </button>
        </nav>
      </header>
      {message && (
        <div className="notice error" role="alert">
          <span>{message}</span>
          {error && (
            <button aria-label="关闭错误" onClick={() => setError(null)}>
              <Icon name="close" />
            </button>
          )}
        </div>
      )}
      {notice && (
        <div className="notice" role="status">
          {notice}
        </div>
      )}
      <div className="workspace-heading">
        <span>
          {song ? "可视化编曲" : "选择音乐生成乐谱，或打开已有的 JSON 乐谱"}
        </span>
        <div>
          {generation && (
            <button className="text-button" onClick={() => setModal("details")}>
              生成详情
            </button>
          )}
          <button
            className="text-button"
            disabled={busy}
            onClick={() => replace(() => void load(loadExample))}
          >
            打开内置示例
          </button>
        </div>
      </div>
      <ScoreEditor
        key={session}
        song={song}
        engine={engine}
        state={state}
        disabled={busy || !!modal || !!pendingAction}
        dirty={dirty}
        onChange={edit}
        transportPosition={transportPosition}
      />
      <Transport
        song={song}
        engine={engine}
        state={state}
        onPosition={(seconds) => setTransportPosition({ seconds })}
      />
      {modal === "generate" && (
        <GenerateScore
          onClose={() => setModal(null)}
          onGenerated={(result) => {
            install(result.arrangement.loaded, result);
            setModal(null);
          }}
        />
      )}
      {modal === "info" && song && (
        <Modal title="乐谱信息" onClose={() => setModal(null)}>
          <dl>
            <dt>名称</dt>
            <dd>{song.score.metadata.name}</dd>
            <dt>来源</dt>
            <dd>{song.source}</dd>
            <dt>格式</dt>
            <dd>
              Score {song.score.format_version} · {song.score.timeline.ppq} PPQ
            </dd>
            <dt>乐器</dt>
            <dd>
              {song.score.instrument.name} ·{" "}
              {song.score.instrument.playback_mode}
            </dd>
            <dt>用户文本</dt>
            <dd>{song.score.text ?? "无"}</dd>
          </dl>
          <p className="muted">
            网格用于编辑和参考，不代表已识别原曲的小节结构。
          </p>
        </Modal>
      )}
      {modal === "details" && generation && (
        <Modal
          title="生成详情"
          wide
          onClose={() => {
            if (!busy) setModal(null);
          }}
        >
          <p className="muted">
            这里记录生成时的转换结果。手动编辑不会改写原始分析和生成报告。
          </p>
          {error && (
            <p role="alert" className="notice error">
              {error}
            </p>
          )}
          {notice && <p role="status">{notice}</p>}
          <details open>
            <summary>编曲结果</summary>
            <button
              disabled={busy || edited}
              title={
                edited
                  ? "乐谱已编辑，请使用顶部的另存乐谱保存当前版本"
                  : undefined
              }
              onClick={() => void saveReport("arrangement")}
            >
              保存生成乐谱及报告
            </button>
            {edited && (
              <p className="muted">
                当前乐谱已编辑，请使用“另存乐谱”保存当前版本。
              </p>
            )}
            <ArrangementReport report={generation.arrangement.report} />
          </details>
          <details>
            <summary>源音乐分析</summary>
            <MusicAnalysis loaded={generation.analysis} />
          </details>
          {generation.audio && (
            <details>
              <summary>音频识别详情</summary>
              <p>
                {generation.audio.report.notes.length} 个候选音符 ·{" "}
                {generation.audio.report.duration_seconds.toFixed(2)} 秒
              </p>
              <button disabled={busy} onClick={() => void saveReport("audio")}>
                保存转录 MIDI 和报告
              </button>
              <ul>
                {generation.audio.report.diagnostics.map((d, i) => (
                  <li key={i}>{d}</li>
                ))}
              </ul>
            </details>
          )}
        </Modal>
      )}
      {modal === "settings" && (
        <Modal title="试听设置" onClose={() => setModal(null)}>
          <label className="setting">
            音量{" "}
            <input
              aria-label="试听音量"
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={volume}
              onChange={(e) => {
                const value = Number(e.target.value);
                setVolume(value);
                engine.setVolume(value);
              }}
            />
            <span>{Math.round(volume * 100)}%</span>
          </label>
          <p>使用本地合成音色试听，速度调整保持音高不变。</p>
          <p className="muted">试听音色不模拟游戏原声。</p>
        </Modal>
      )}
      {pendingAction && (
        <Modal
          title="保留当前乐谱的修改？"
          onClose={() => {
            if (!busy) setPendingAction(null);
          }}
        >
          <p>当前乐谱尚未保存，打开其他音乐后会替换它。</p>
          {error && (
            <p className="notice error" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-actions">
            <button disabled={busy} onClick={() => setPendingAction(null)}>
              继续编辑
            </button>
            <button
              disabled={busy}
              onClick={() => {
                const action = pendingAction;
                setPendingAction(null);
                action();
              }}
            >
              放弃并继续
            </button>
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void save().then((saved) => {
                  if (saved) {
                    const action = pendingAction;
                    setPendingAction(null);
                    action();
                  }
                })
              }
            >
              保存并继续
            </button>
          </div>
        </Modal>
      )}
    </main>
  );
}
