import { memo, useMemo, useState } from "react";
import type { ArrangementReport as Report } from "../arrangement/model.ts";

const percent = (n: number | null) => n === null ? "未知 / 不适用" : `${(n * 100).toFixed(1)}%`;
const reasons: Record<string, string> = {
  midi_out_of_range: "超出 MIDI 0–127",
  below_instrument_range: "低于乐器音域",
  above_instrument_range: "高于乐器音域",
  not_in_instrument_keys: "不在实际键位列表中",
};

export const ArrangementReport = memo(function ArrangementReport({ report: r }: { report: Report }) {
  const [page, setPage] = useState(0);
  const dropped = useMemo(() => r.note_mappings.filter(n => !n.mapped), [r]);
  const pages = Math.max(1, Math.ceil(dropped.length / 20));
  const current = Math.min(page, pages - 1);
  const selected = r.candidates.find(c => c.selected);
  return <section className="music-analysis" aria-label="编曲报告">
    <p className="analysis-source">{r.source_file} · {r.target_instrument}</p>
    <div className="analysis-metrics">
      <div><strong>{r.transpose_semitones ?? "—"}</strong><span>全局移调（半音）</span></div>
      <div><strong>{r.kept} / {r.original_notes}</strong><span>保留音符</span></div>
      <div><strong>{percent(selected?.melody_retention ?? null)}</strong><span>旋律候选保留 {selected?.melody_kept ?? "未知"} / {selected?.melody_total ?? "未知"}</span></div>
      <div><strong>{percent(selected?.accompaniment_retention ?? null)}</strong><span>伴奏候选保留 {selected?.accompaniment_kept ?? 0} / {selected?.accompaniment_total ?? 0}</span></div>
    </div>
    <p>舍弃 {r.dropped} 个音符 · 总保留比例 {percent(r.retention)}</p>
    <p className="muted">{r.melody.track === null ? "旋律 Unknown：以全部音符保留为首要目标。" : `旋律候选：轨道 ${r.melody.track}。`} 角色来自未校准启发式，不能视为音乐事实。</p>
    <details><summary>选择依据与诊断</summary><p>{r.melody.reason}</p>{r.diagnostics.map((d, i) => <p key={i}>{d}</p>)}</details>
    <div className="analysis-table-scroll"><table><thead><tr><th>源轨道（0 起）</th><th>角色候选</th><th>保留</th><th>舍弃</th><th>比例</th></tr></thead><tbody>
      {r.tracks.map(t => <tr key={t.source_track}><td>{t.source_track} · {t.name ?? "未命名"}</td><td>{t.phase3_role.role}</td><td>{t.kept}</td><td>{t.dropped}</td><td>{percent(t.retention)}</td></tr>)}
    </tbody></table></div>
    <details open><summary>逐音舍弃明细（{dropped.length}）</summary>
      <div className="analysis-table-scroll"><table><thead><tr><th>源轨道 / 音符</th><th>MIDI 源 → 目标</th><th>源 tick / 时长</th><th>投影 tick / 时长</th><th>原因</th></tr></thead><tbody>
        {dropped.slice(current * 20, current * 20 + 20).map(n => <tr key={n.source_note_id}><td>{n.source_track} / {n.source_note_id}</td><td>{n.source_midi} → {n.target_midi}</td><td>{n.source_start_tick} / {n.source_duration_ticks}</td><td>{n.projected_start_tick} / {n.projected_duration_ticks}</td><td>{reasons[n.drop_reason ?? ""] ?? n.drop_reason}</td></tr>)}
      </tbody></table>{!dropped.length && <p>没有舍弃音符。</p>}</div>
      {pages > 1 && <div className="analysis-pagination"><button disabled={current === 0} onClick={() => setPage(current - 1)}>上一页</button><span>{current + 1} / {pages}</span><button disabled={current + 1 === pages} onClick={() => setPage(current + 1)}>下一页</button></div>}
    </details>
    <details><summary>全部移调候选（{r.candidates.length}）</summary><div className="analysis-table-scroll"><table><thead><tr><th>半音</th><th>旋律</th><th>全部</th><th>伴奏</th><th>舍弃</th></tr></thead><tbody>
      {r.candidates.map(c => <tr key={c.transpose_semitones}><td>{c.transpose_semitones}{c.selected ? " ✓" : ""}</td><td>{c.melody_kept ?? "未知"}</td><td>{c.total_kept}</td><td>{c.accompaniment_kept}</td><td>{c.dropped}</td></tr>)}
    </tbody></table></div></details>
    <p>{r.score_validation && r.time_preserved && r.pitch_mapping_valid ? "Score 验证、投影时间保持、键位映射检查通过。" : "尚未通过全部输出检查。"}</p>
    {r.score_file && <p className="analysis-source">Score：{r.score_file}</p>}
    {r.report_file && <p className="analysis-source">完整报告：{r.report_file}</p>}
  </section>;
});
