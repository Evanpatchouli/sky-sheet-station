import { useEffect, useRef, useState } from "react";
import { Modal } from "./Modal.tsx";
import { audioOptionsError, defaultAudioOptions, progressPercent, stageLabels } from "../audio/model.ts";
import type { AudioProgress, LoadedAudio } from "../audio/model.ts";
import { activateTranscription, cancelAudio, saveTranscription, transcribeAudio } from "../services/desktop.ts";
import { analysisErrorMessage } from "../analysis/model.ts";

export function AudioTranscription({result, onResult, onContinue, onBusy, onClose}: {
  result: LoadedAudio | null; onResult: (result: LoadedAudio) => void;
  onContinue: (result: LoadedAudio) => void; onBusy: (busy: boolean) => void; onClose: () => void;
}) {
  const [options, setOptions] = useState(defaultAudioOptions);
  const [progress, setProgress] = useState<AudioProgress | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const current = useRef<string | null>(null);
  useEffect(() => () => {
    const id = current.current;
    current.current = null;
    if (id) { void cancelAudio(id).catch(() => {}); onBusy(false); }
  }, [onBusy]);
  async function start() {
    const invalid = audioOptionsError(options);
    if (invalid) { setError(invalid); return; }
    const id = crypto.randomUUID(); current.current = id;
    setWorking(true); onBusy(true); setError(null); setSaved(null);
    setProgress({stage: "pick", completed: 0, total: null});
    try {
      const loaded = await transcribeAudio(id, options, next => {
        if (current.current !== id) return;
        setProgress(previous => previous?.stage === next.stage && previous.completed > next.completed ? previous : next);
      });
      if (loaded && current.current === id) { onResult(loaded); setPage(0); }
    } catch (e) { if (current.current === id) setError(analysisErrorMessage(e)); }
    finally { if (current.current === id) { current.current = null; setWorking(false); onBusy(false); setProgress(null); } }
  }
  async function cancel() {
    const id = current.current;
    if (!id) return;
    // Stop accepting progress/results before awaiting process termination.
    current.current = null;
    try { await cancelAudio(id); setError("已取消转录，原有乐谱和上次结果已保留。"); }
    catch (e) { setError(analysisErrorMessage(e)); }
    finally { setWorking(false); onBusy(false); setProgress(null); }
  }
  async function save() {
    if (!result) return;
    setWorking(true); onBusy(true); setError(null);
    try { const path = await saveTranscription(result.token); if (path) setSaved(path); }
    catch (e) { setError(analysisErrorMessage(e)); }
    finally { setWorking(false); onBusy(false); }
  }
  async function continueToArrangement() {
    if (!result) return;
    setWorking(true); onBusy(true); setError(null);
    try { await activateTranscription(result.token); onContinue(result); }
    catch (e) { setError(analysisErrorMessage(e)); }
    finally { setWorking(false); onBusy(false); }
  }
  const percent = progress ? progressPercent(progress) : undefined;
  const notes = result?.report.notes ?? [];
  return <Modal title="音频转录 · Audio to MIDI" wide onClose={() => { if (working) { if (current.current) void cancel().then(onClose); } else onClose(); }}>
    <p>选择 MP3、WAV 或 FLAC，在本机识别音高、起止时间与估计力度。支持最长 10 分钟、128 MiB；音频无需上传。</p>
    <div className="arrangement-controls">
      <label>起音阈值<input aria-label="起音阈值" type="number" min="0.05" max="0.95" step="0.05" value={Number.isNaN(options.onset_threshold) ? '' : options.onset_threshold} disabled={working} onChange={e=>setOptions({...options,onset_threshold:e.target.value===''?NaN:Number(e.target.value)})}/></label>
      <label>持续阈值<input aria-label="持续阈值" type="number" min="0.05" max="0.95" step="0.05" value={Number.isNaN(options.frame_threshold) ? '' : options.frame_threshold} disabled={working} onChange={e=>setOptions({...options,frame_threshold:e.target.value===''?NaN:Number(e.target.value)})}/></label>
      <label>最短音符 / ms<input aria-label="最短音符" type="number" min="30" max="1000" step="10" value={Number.isNaN(options.minimum_note_ms) ? '' : options.minimum_note_ms} disabled={working} onChange={e=>setOptions({...options,minimum_note_ms:e.target.value===''?NaN:Number(e.target.value)})}/></label>
      <button className="primary" disabled={working} onClick={()=>void start()}>选择音频并转录</button>
      {working && current.current && <button onClick={()=>void cancel()}>取消转录</button>}
    </div>
    <p className="muted">较低阈值可找回弱音，也可能增加误识别。混音、鼓声、混响及人声会影响准确性；请预览后再用于编曲。</p>
    {progress && <div className="audio-progress" role="status" aria-live="polite">
      <strong>{stageLabels[progress.stage] ?? progress.stage}{percent !== undefined ? ` · ${percent}%` : '…'}</strong>
      <progress max={100} value={percent} aria-label="当前阶段进度" />
      {progress.stage === "transcribe" && <span>{progress.completed} / {progress.total} 个音频窗口</span>}
    </div>}
    {error && <p className="notice error" role="alert">{error}</p>}
    {result && <>
      <div className="audio-file"><span>转录来源</span><strong>{result.source}</strong></div>
      <p><strong>{notes.length} 个候选音符</strong> · {result.report.duration_seconds.toFixed(2)} 秒 · {result.report.source_sample_rate} Hz / {result.report.source_channels} 声道</p>
      <div className="arrangement-controls">
        <button disabled={working} onClick={()=>void save()}>保存转录 MIDI 和报告</button>
        <button className="primary" disabled={working || notes.length === 0} onClick={()=>void continueToArrangement()}>进入 Sky 编曲与预览试听</button>
      </div>
      {saved && <p role="status">已保存：{saved}（含独立转录报告）</p>}
      {notes.length === 0 && <p role="status">没有检测到可用音符。可保存空 MIDI，或调整阈值后重新转录。</p>}
      <p className="muted">120 BPM / 4/4 仅为时间参考，并非测得节拍。力度是模型估计，保存在转录 MIDI 中；Sky Score 沿用现有格式。以下时间以音频起点为零。</p>
      {notes.length > 0 && <>
        <div className="music-analysis analysis-table-scroll"><table><thead><tr><th>音高 / MIDI</th><th>起始 / 秒</th><th>结束 / 秒</th><th>力度</th><th>模型激活</th></tr></thead><tbody>
          {notes.slice(page*50, (page+1)*50).map((n,i)=><tr key={page*50+i}><td>{n.pitch}</td><td>{(n.start_sample/result.report.sample_rate).toFixed(3)}</td><td>{(n.end_sample/result.report.sample_rate).toFixed(3)}</td><td>{n.velocity}</td><td>{n.activation.toFixed(3)}</td></tr>)}
        </tbody></table></div>
        <div className="analysis-pagination"><button disabled={page===0} onClick={()=>setPage(page-1)}>上一页</button><span>{page+1} / {Math.ceil(notes.length/50)}</span><button disabled={(page+1)*50>=notes.length} onClick={()=>setPage(page+1)}>下一页</button></div>
      </>}
      <details><summary>转录 Report · 方法与诊断</summary><p className="muted">{result.report.engine}</p><ul>{result.report.diagnostics.map((d,i)=><li key={i}>{d}</li>)}</ul></details>
    </>}
  </Modal>;
}
