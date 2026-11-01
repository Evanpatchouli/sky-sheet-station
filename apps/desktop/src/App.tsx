import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { Icon } from "./components/Icon.tsx";
import { Keyboard } from "./components/Keyboard.tsx";
import { PianoRoll, TRACK_COLORS } from "./components/PianoRoll.tsx";
import { Transport } from "./components/Transport.tsx";
import { Modal } from "./components/Modal.tsx";
import { ImportReport } from "./components/ImportReport.tsx";
import { MusicAnalysis } from "./components/MusicAnalysis.tsx";
import { ArrangementReport } from "./components/ArrangementReport.tsx";
import { AudioTranscription } from "./components/AudioTranscription.tsx";
import type { LoadedAudio } from "./audio/model.ts";
import type { ArrangementReport as ArrangementDiagnostic, LoadedArrangement } from "./arrangement/model.ts";
import { defaultArrangementOptions, optionsError } from "./arrangement/model.ts";
import type { ArrangementOptions } from "./arrangement/model.ts";
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
  saveArrangement,
  cancelArrangement,
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
  const [audio, setAudio] = useState<LoadedAudio | null>(null);
  const [analysis, setAnalysis] = useState<LoadedAnalysis | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [arrangement, setArrangement] = useState<LoadedArrangement | null>(null);
  const [arrangementReport, setArrangementReport] = useState<ArrangementDiagnostic | null>(null);
  const [arrangementError, setArrangementError] = useState<string | null>(null);
  const [arrangementMode, setArrangementMode] = useState("auto");
  const [transpose, setTranspose] = useState("0");
  const [arrangeOptions, setArrangeOptions] = useState(defaultArrangementOptions);
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
    engine.pause();
    setModal("audio");
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
    const invalid=optionsError(arrangeOptions);
    if(invalid) {setArrangementError(invalid);return;}
    const shift = arrangementMode === "auto" ? null : Number(transpose);
    if (shift !== null && (transpose.trim() === "" || !Number.isInteger(shift) || shift < -127 || shift > 127)) {
      setArrangementError("移调必须是 -127..127 的整数。"); return;
    }
    const id = ++request.current;
    engine.pause(); setBusy(true); setArrangementError(null);
    try {
      const result = await arrangeMidi(analysis.report.source_document_id, shift, arrangeOptions);
      if (result && id === request.current) { setArrangement(result); setArrangementReport(result.report); }
    } catch (e) {
      if (id === request.current) {
        setArrangementError(analysisErrorMessage(e));
        if (e && typeof e === "object" && "report" in e && e.report) setArrangementReport(e.report as ArrangementDiagnostic);
        setArrangement(null);
      }
    } finally { if (id === request.current) setBusy(false); }
  }
  async function saveArranged() {
    if(!arrangement) return;
    const id=++request.current; setBusy(true);setArrangementError(null);
    try {const report=await saveArrangement(arrangement.token);if(report&&id===request.current){setArrangementReport(report);setArrangement({...arrangement,report});}}
    catch(e){if(id===request.current)setArrangementError(analysisErrorMessage(e));}
    finally {if(id===request.current)setBusy(false);}
  }
  function closeArrangement() {
    if(busy){++request.current;setBusy(false);void cancelArrangement().catch(e=>setArrangementError(analysisErrorMessage(e)));}
    setModal(null);
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
                ? "支持 Score JSON、MIDI 及音频转录（MP3 / WAV / FLAC）"
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
      {modal === "arrange" && <Modal title="Sky 15-key 编曲" wide onClose={closeArrangement}>
        <p>目标乐器：sky-15 · C4–C6 · 15 个实际键位 · 上下文八度适配</p>
        {!analysis ? <><p>先分析 MIDI，再选择自动适配或固定移调。</p><button className="primary" disabled={busy} onClick={() => void analyze()}>选择 MIDI 并分析</button></> : <>
          <p className="analysis-source">来源：{analysis.source}</p>
          <div className="arrangement-controls">
            <label>编曲模式 <select aria-label="编曲模式" value={arrangeOptions.mode} disabled={busy} onChange={e=>setArrangeOptions({...arrangeOptions,mode:e.target.value as ArrangementOptions["mode"]})}><option value="melody-first">旋律优先</option><option value="simple-accompaniment">简易伴奏</option><option value="polyphonic">多声部优先</option></select></label>
            <label><input type="radio" name="arrangement-mode" checked={arrangementMode === "auto"} disabled={busy} onChange={() => setArrangementMode("auto")} /> 自动适配</label>
            <label><input type="radio" name="arrangement-mode" checked={arrangementMode === "fixed"} disabled={busy} onChange={() => setArrangementMode("fixed")} /> 固定移调</label>
            <label>半音数 <input aria-label="移调半音数" type="number" min="-127" max="127" step="1" value={transpose} disabled={busy || arrangementMode !== "fixed"} onChange={e => setTranspose(e.target.value)} /></label>
            <label><input type="checkbox" checked={arrangeOptions.allow_octave_shifts} disabled={busy} onChange={e=>setArrangeOptions({...arrangeOptions,allow_octave_shifts:e.target.checked})}/>允许声部八度调整</label>
            {([['melody_priority','旋律优先级'],['bass_priority','低音优先级'],['accompaniment_priority','伴奏优先级']] as const).map(([key,label])=><label key={key}>{label}<input aria-label={label} type="number" min="0" max="5" step="1" value={Number.isNaN(arrangeOptions[key])?'':arrangeOptions[key]} disabled={busy} onChange={e=>setArrangeOptions({...arrangeOptions,[key]:e.target.value===''?NaN:Number(e.target.value)})}/></label>)}
            <label>最大同时发声数<input aria-label="最大同时发声数" type="number" min="1" max="128" step="1" placeholder="不限" value={arrangeOptions.maximum_polyphony??''} disabled={busy} onChange={e=>setArrangeOptions({...arrangeOptions,maximum_polyphony:e.target.value===''?null:Number(e.target.value)})}/></label>
            <button className="primary" disabled={busy} onClick={() => void arrange()}>生成编曲预览</button>
            <button onClick={closeArrangement}>{busy?'取消等待并返回':'返回播放器'}</button>
          </div>
          <p className="muted">优先级 0–5 影响有取舍时的排序，旋律始终属于受保护层。复音上限是用户约束；留空不限。先预览和试听，再保存 Score 与独立报告，已有文件不会覆盖。取消等待会忽略结果；后台 CLI 可完成临时计算。</p>
        </>}
        {busy && <p role="status">正在通过 CLI 编曲并验证…</p>}
        {arrangementError && <p className="notice error" role="alert">{arrangementError}</p>}
        {arrangement && <button className="primary" disabled={busy} onClick={() => { void load(async () => arrangement.loaded); setModal(null); }}>加载生成乐谱并预览试听</button>}
        {arrangement && <button disabled={busy} onClick={()=>void saveArranged()}>保存 Score 和报告</button>}
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
        <AudioTranscription result={audio} onBusy={setBusy} onClose={()=>setModal(null)}
          onResult={result=>{setAudio(result);setAnalysis(result.analysis);setArrangement(null);setArrangementReport(null);setArrangementError(null);}}
          onContinue={result=>{setAnalysis(result.analysis);setModal("arrange");}} />
      )}
    </main>
  );
}
