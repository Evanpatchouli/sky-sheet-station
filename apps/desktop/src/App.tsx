import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { Icon } from "./components/Icon.tsx";
import { Keyboard } from "./components/Keyboard.tsx";
import { PianoRoll, TRACK_COLORS } from "./components/PianoRoll.tsx";
import { Transport } from "./components/Transport.tsx";
import { Modal } from "./components/Modal.tsx";
import { ImportReport } from "./components/ImportReport.tsx";
import { MusicAnalysis } from "./components/MusicAnalysis.tsx";
import { ArrangementReport } from "./components/ArrangementReport.tsx";
import type { ArrangementReport as ArrangementDiagnostic, LoadedArrangement } from "./arrangement/model.ts";
import { analysisErrorMessage } from "./analysis/model.ts";
import type { LoadedAnalysis } from "./analysis/model.ts";
import { PlayerEngine } from "./player/engine.ts";
import { prepareScore } from "./player/model.ts";
import type {
  LoadedScore,
  PreparedScore,
  ImportReport as Report,
} from "./player/model.ts";
import {
  desktopAvailable,
  loadExample,
  openScore,
  openMidi,
  analyzeMidi,
  arrangeMidi,
  selectAudio,
} from "./services/desktop.ts";

export default function App() {
  const [engine] = useState(() => new PlayerEngine());
  const state = useSyncExternalStore(engine.subscribe, engine.snapshot);
  const [song, setSong] = useState<PreparedScore | null>(null);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<"info" | "settings" | "audio" | "import" | "analysis" | "arrange" | null>(
    null,
  );
  const [audio, setAudio] = useState<string | null>(null);
  const [analysis, setAnalysis] = useState<LoadedAnalysis | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [arrangement, setArrangement] = useState<LoadedArrangement | null>(null);
  const [arrangementReport, setArrangementReport] = useState<ArrangementDiagnostic | null>(null);
  const [arrangementError, setArrangementError] = useState<string | null>(null);
  const [arrangementMode, setArrangementMode] = useState("auto");
  const [transpose, setTranspose] = useState("0");
  const [importReport, setImportReport] = useState<Report | null>(null);
  const [zoom, setZoom] = useState(1),
    [follow, setFollow] = useState(true),
    [volume, setVolume] = useState(0.65);
  const request = useRef(0);
  useEffect(() => () => engine.dispose(), [engine]);
  async function load(action: () => Promise<LoadedScore | null>) {
    const id = ++request.current;
    engine.pause();
    setBusy(true);
    setError(null);
    setImportReport(null);
    try {
      const loaded = await action();
      if (loaded && id === request.current) {
        const prepared = prepareScore(loaded);
        engine.load(prepared);
        setSong(prepared);
        setImportReport(loaded.import_report ?? null);
      }
    } catch (e) {
      if (id === request.current) {
        if (e && typeof e === "object" && "message" in e) {
          setError(String(e.message));
          if ("report" in e && e.report) setImportReport(e.report as Report);
        } else setError(String(e));
      }
    } finally {
      if (id === request.current) setBusy(false);
    }
  }
  async function pickAudio() {
    setModal("audio");
    try {
      const path = await selectAudio();
      if (path) setAudio(path);
    } catch (e) {
      setError(String(e));
    }
  }
  async function analyze() {
    const id = ++request.current;
    engine.pause();
    setBusy(true); setAnalysisBusy(true); setAnalysisError(null); setModal("analysis");
    try {
      const loaded = await analyzeMidi();
      if (loaded && id === request.current) { setAnalysis(loaded); setArrangement(null); setArrangementReport(null); setArrangementError(null); }
    } catch (error) {
      if (id === request.current) setAnalysisError(analysisErrorMessage(error));
    } finally {
      if (id === request.current) { setBusy(false); setAnalysisBusy(false); }
    }
  }
  async function arrange() {
    if (!analysis) return;
    const shift = arrangementMode === "auto" ? null : Number(transpose);
    if (shift !== null && (transpose.trim() === "" || !Number.isInteger(shift) || shift < -127 || shift > 127)) {
      setArrangementError("移调必须是 -127..127 的整数。"); return;
    }
    const id = ++request.current;
    engine.pause(); setBusy(true); setArrangementError(null);
    try {
      const result = await arrangeMidi(analysis.report.source_document_id, shift);
      if (result && id === request.current) { setArrangement(result); setArrangementReport(result.report); }
    } catch (e) {
      if (id === request.current) {
        setArrangementError(analysisErrorMessage(e));
        if (e && typeof e === "object" && "report" in e && e.report) setArrangementReport(e.report as ArrangementDiagnostic);
        setArrangement(null);
      }
    } finally { if (id === request.current) setBusy(false); }
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
              ? `${song.score.tracks.length} 轨 · ${song.notes.length} 音符`
              : "从一份真实乐谱开始"}
          </span>
        </div>
        <nav aria-label="文件与设置">
          <button disabled={busy} onClick={() => void load(openScore)}>
            <Icon name="open" />
            <span>打开乐谱</span>
          </button>
          <button disabled={busy} onClick={() => void pickAudio()}>
            <Icon name="audio" />
            <span>音频导入</span>
          </button>
          <button
            aria-label="打开 MIDI"
            title="打开 MIDI"
            disabled={busy}
            onClick={() => void load(openMidi)}
          >
            <Icon name="open" />
            <span>打开 MIDI</span>
          </button>
          {importReport && (
            <button
              aria-label="导入报告"
              title="导入报告"
              onClick={() => setModal("import")}
            >
              <Icon name="info" />
              <span>导入报告</span>
            </button>
          )}
          <button aria-label="分析 MIDI" disabled={busy} onClick={() => void analyze()}><Icon name="info" /><span>分析 MIDI</span></button>
          {analysis && <button aria-label="分析结果" onClick={() => setModal("analysis")}><Icon name="info" /><span>分析结果</span></button>}
          <button aria-label="Sky 编曲" disabled={busy} onClick={() => setModal("arrange")}><Icon name="audio" /><span>Sky 编曲</span></button>
          <button disabled={!song} onClick={() => setModal("info")}>
            <Icon name="info" />
            <span>乐谱信息</span>
          </button>
          <button onClick={() => setModal("settings")}>
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
      <div className="roll-toolbar">
        <div className="legend">
          {song ? (
            song.score.tracks.map((track, i) => (
              <span key={track.id}>
                <i
                  style={{ background: TRACK_COLORS[i % TRACK_COLORS.length] }}
                />
                {track.name}
              </span>
            ))
          ) : (
            <span>Score JSON · 乐谱视图</span>
          )}
        </div>
        <div className="view-controls">
          <label>
            缩放{" "}
            <input
              aria-label="缩放"
              type="range"
              min="0.25"
              max="4"
              step="0.05"
              value={zoom}
              onChange={(e) => setZoom(Number(e.target.value))}
            />
          </label>
          <span>{Math.round(zoom * 100)}%</span>
          <label>
            <input
              type="checkbox"
              checked={follow}
              onChange={(e) => setFollow(e.target.checked)}
            />
            跟随播放
          </label>
        </div>
      </div>
      <PianoRoll
        song={song}
        position={state.position}
        playing={state.status === "playing"}
        zoom={zoom}
        follow={follow}
        onSeek={(seconds) => engine.seek(seconds)}
      />
      <div className="score-status">
        <span>
          {busy
            ? "正在通过 CLI 读取并验证…"
            : song
              ? `${song.score.timeline.tempo} BPM · ${song.score.timeline.time_signature.numerator}/${song.score.timeline.time_signature.denominator} 参考拍号 · ${song.score.timeline.ppq} PPQ`
              : desktopAvailable()
                ? "支持 Score JSON 与 MIDI（.mid / .midi）"
                : "浏览器预览 · 文件工作流请使用桌面应用"}
        </span>
        <button
          disabled={busy}
          className="text-button"
          onClick={() => void load(loadExample)}
        >
          打开内置示例
        </button>
      </div>
      <Keyboard song={song} active={state.keys} />
      <Transport song={song} engine={engine} state={state} />
      {modal === "info" && song && (
        <Modal title="乐谱信息" onClose={() => setModal(null)}>
          <dl>
            <dt>名称</dt>
            <dd>{song.score.metadata.name}</dd>
            <dt>乐谱 ID</dt>
            <dd>{song.score.metadata.id}</dd>
            <dt>来源</dt>
            <dd>{song.source}</dd>
            <dt>格式</dt>
            <dd>Score {song.score.format_version}</dd>
            <dt>乐器</dt>
            <dd>
              {song.score.instrument.name} ·{" "}
              {song.score.instrument.playback_mode}
            </dd>
            <dt>内容</dt>
            <dd>
              {song.score.tracks.length} 轨 / {song.notes.length} 音符 /{" "}
              {song.endTick} ticks
            </dd>
            <dt>用户文本</dt>
            <dd>{song.score.text ?? "无"}</dd>
          </dl>
          <p className="muted">
            拍号网格仅是名义参考，不表示已识别原曲的小节结构。
          </p>
        </Modal>
      )}
      {modal === "import" && importReport && (
        <Modal title="MIDI 导入报告" onClose={() => setModal(null)}>
          <ImportReport report={importReport} />
        </Modal>
      )}
      {modal === "analysis" && <Modal title="音乐分析" wide onClose={() => setModal(null)}>
        {analysisBusy && <p role="status">正在读取 MIDI 并通过 CLI 分析…</p>}
        {analysisError && <p className="notice error" role="alert">{analysisError}</p>}
        {analysis && <button className="primary" disabled={busy} onClick={() => setModal("arrange")}>适配为 Sky 乐谱</button>}
        {analysis ? <MusicAnalysis loaded={analysis} /> : !analysisBusy && <p>选择 MIDI 文件以查看原始轨道、音域与旋律候选。</p>}
      </Modal>}
      {modal === "arrange" && <Modal title="Sky 15-key 编曲" wide onClose={() => setModal(null)}>
        <p>目标乐器：sky-15 · C4–C6 · 全曲统一移调</p>
        {!analysis ? <><p>先分析 MIDI，再选择自动适配或固定移调。</p><button className="primary" disabled={busy} onClick={() => void analyze()}>选择 MIDI 并分析</button></> : <>
          <p className="analysis-source">来源：{analysis.source}</p>
          <div className="arrangement-controls">
            <label><input type="radio" name="arrangement-mode" checked={arrangementMode === "auto"} disabled={busy} onChange={() => setArrangementMode("auto")} /> 自动适配</label>
            <label><input type="radio" name="arrangement-mode" checked={arrangementMode === "fixed"} disabled={busy} onChange={() => setArrangementMode("fixed")} /> 固定移调</label>
            <label>半音数 <input aria-label="移调半音数" type="number" min="-127" max="127" step="1" value={transpose} disabled={busy || arrangementMode !== "fixed"} onChange={e => setTranspose(e.target.value)} /></label>
            <button className="primary" disabled={busy} onClick={() => void arrange()}>生成并保存 Score</button>
          </div>
          <p className="muted">将同时保存独立编曲报告；已有文件不会覆盖。无法映射的音符会在报告中逐一列出。</p>
        </>}
        {busy && <p role="status">正在通过 CLI 编曲并验证…</p>}
        {arrangementError && <p className="notice error" role="alert">{arrangementError}</p>}
        {arrangement && <button className="primary" disabled={busy} onClick={() => { void load(async () => arrangement.loaded); setModal(null); }}>加载生成乐谱并预览试听</button>}
        {arrangementReport && <ArrangementReport key={`${arrangementReport.source_document_id}-${arrangementReport.transpose_semitones}`} report={arrangementReport} />}
      </Modal>}
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
                const next = Number(e.target.value);
                setVolume(next);
                engine.setVolume(next);
              }}
            />
            <span>{Math.round(volume * 100)}%</span>
          </label>
          <p>使用本地合成音色预听 Score。速度调整保持音高不变。</p>
          <p className="muted">
            音高使用乐谱逻辑映射，不代表游戏内实际音高；不会发送任何游戏按键。
          </p>
        </Modal>
      )}
      {modal === "audio" && (
        <Modal title="音频转换尚未接通" onClose={() => setModal(null)}>
          <p>
            当前支持 MIDI 导入与 Score JSON，尚不能将 MP3、WAV 或 FLAC 转为乐谱。
          </p>
          {audio && (
            <div className="audio-file">
              <span>已选择，仅保留文件路径</span>
              <strong>{audio}</strong>
            </div>
          )}
          <p className="muted">
            请打开已有的 Score JSON
            进行可视化与试听。此入口不会生成音符，也不会把原始音频作为乐谱播放。
          </p>
          <button
            className="primary"
            onClick={() => {
              setModal(null);
              void load(openScore);
            }}
          >
            <Icon name="open" />
            打开 Score JSON
          </button>
        </Modal>
      )}
    </main>
  );
}
