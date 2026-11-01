import { memo, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { LoadedAnalysis } from "../analysis/model.ts";

function PagedTable<T>({ rows, headings, render }: { rows: T[]; headings: string[]; render: (row: T, index: number) => ReactNode }) {
  const [page, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(rows.length / 20));
  const current = Math.min(page, pages - 1);
  return <>
    <div className="analysis-table-scroll"><table><thead><tr>{headings.map(h => <th key={h}>{h}</th>)}</tr></thead>
      <tbody>{rows.slice(current * 20, current * 20 + 20).map((r, i) => render(r, current * 20 + i))}</tbody></table>
      {rows.length === 0 && <p className="muted">无数据</p>}
    </div>
    {pages > 1 && <div className="analysis-pagination"><button disabled={current === 0} onClick={() => setPage(current - 1)}>上一页</button><span>{current + 1} / {pages} · {rows.length} 条</span><button disabled={current + 1 === pages} onClick={() => setPage(current + 1)}>下一页</button></div>}
  </>;
}
const roles: Record<string, string> = { melody: "旋律候选", bass: "低音候选", harmony: "和声候选", accompaniment: "伴奏候选", rhythm: "节奏候选", unknown: "未知" };
const origin = (value: string) => value === "protocol_default" ? "协议默认" : value === "source" ? "MIDI 明确提供" : value;
const decimal = (value: number | null) => value === null ? "未知" : value.toFixed(3);

export const MusicAnalysis = memo(function MusicAnalysis({ loaded }: { loaded: LoadedAnalysis }) {
  const r = loaded.report;
  const roleMap = useMemo(() => new Map(r.track_roles.map(t => [t.track, t])), [r]);
  const candidateMap = useMemo(() => new Map(r.melody_candidates.map(c => [c.id, c])), [r]);
  return <section className="music-analysis" aria-label="音乐分析结果">
    <p className="analysis-source">{loaded.source}</p>
    <div className="analysis-metrics">
      <div><strong>{r.note_statistics.total_notes}</strong><span>原始音符</span></div>
      <div><strong>{r.structure_analysis.tracks.length}</strong><span>原始轨道</span></div>
      <div><strong>{r.note_statistics.max_polyphony}</strong><span>最大复音</span></div>
      <div><strong>{r.source.ppq}</strong><span>原始 PPQ</span></div>
    </div>
    <p className="muted">分析保留原始音符。候选评分是未校准的启发式指标，不能视为主旋律判定或正确概率。</p>
    <h3>轨道与音域</h3>
    <PagedTable rows={r.structure_analysis.tracks} headings={["轨道", "音符", "音域", "活动 ticks", "音符 / 四分音符", "角色候选"]} render={t => {
      const role = roleMap.get(t.track);
      return <tr key={t.track}><td>{t.track} · {t.name ?? "未命名"}</td><td>{t.note_count}</td><td>{t.pitch_range ? `${t.pitch_range.lowest_name}–${t.pitch_range.highest_name} · ${t.pitch_range.span_semitones} 半音` : "未知"}</td><td>{t.active_ticks}</td><td>{decimal(t.onsets_per_quarter)}</td><td>{roles[role?.role ?? "unknown"]}<details><summary>依据</summary>{role?.candidates.map(c => <p key={c.role}>{roles[c.role] ?? c.role} · {c.score.toFixed(3)}<br />{c.reasons.join("；")}</p>)}</details></td></tr>;
    }} />
    <h3>旋律候选排名</h3>
    <p className="muted">逐轨取最高声部候选得分；同分按原轨道序号排列，空轨道为 0。</p>
    <PagedTable rows={r.track_ranking} headings={["排名", "原轨道", "启发式得分", "分项与不确定性"]} render={t => <tr key={t.track}>
      <td>{t.rank}</td><td>{t.track}</td><td>{t.score.toFixed(3)}</td><td><details><summary>查看分项</summary>
        {t.scoring ? <><p>{t.scoring.eligible ? "参与评分" : "证据不足或打击乐通道；总分为 0"}</p>{t.scoring.components.map(c => <p key={c.name}>{c.name}: {c.raw.toFixed(4)} × {c.weight.toFixed(2)} = {c.contribution.toFixed(4)}</p>)}</> : <p>空轨道</p>}
        {t.best_candidate_id && candidateMap.get(t.best_candidate_id)?.reasons.map((reason, i) => <p key={`r${i}`}>{reason}</p>)}
        {t.best_candidate_id && candidateMap.get(t.best_candidate_id)?.possible_errors.map((reason, i) => <p key={`e${i}`} className="muted">{reason}</p>)}
      </details></td></tr>} />
    <h3>节奏与音区</h3>
    <p>{r.rhythm_analysis.unique_onsets} 个不同起音位置 · {r.note_statistics.overlapping_note_pairs} 对区间重叠音符 · 主要起音间隔占比 {decimal(r.rhythm_analysis.modal_interval_fraction)}</p>
    <p>低 / 中 / 高音区：{r.structure_analysis.registers.low} / {r.structure_analysis.registers.middle} / {r.structure_analysis.registers.high}</p>
    <p className="muted">音区阈值由分析引擎定义：低音区 MIDI &lt;{r.structure_analysis.low_register_end}，中音区 [{r.structure_analysis.low_register_end}, {r.structure_analysis.high_register_start})，高音区 ≥{r.structure_analysis.high_register_start}。这是描述性启发式，不是乐器限制或音乐学绝对标准。</p>
    <details><summary>轨道间节奏与时间重叠</summary>
      {!r.structure_analysis.pairwise_complete && <p>轨道数量超出成对分析预算，请查看诊断。</p>}
      <PagedTable rows={r.structure_analysis.track_pairs} headings={["轨道", "同步起音位置", "区间重叠 ticks", "起音集合交并比"]} render={(t, i) => <tr key={i}><td>{t.tracks.join(" / ")}</td><td>{t.simultaneous_onsets}</td><td>{t.overlap_ticks}</td><td>{decimal(t.onset_jaccard)}</td></tr>} />
    </details>
    <h3>Tempo Map</h3>
    <p>变化 {r.tempo_analysis.change_count} 次 · 主要 BPM {decimal(r.tempo_analysis.principal_bpm)}（按原始 tick 跨度统计）</p>
    <PagedTable rows={r.tempo_analysis.effective_map} headings={["开始 tick", "微秒 / 四分音符", "来源"]} render={(t, i) => <tr key={i}><td>{t.start_tick}</td><td>{t.microseconds_per_quarter}</td><td>{origin(t.origin)}</td></tr>} />
    <h3>Meter Map</h3>
    <PagedTable rows={r.structure_analysis.meter_segments} headings={["开始 tick", "结束 tick", "拍号", "起始名义小节", "来源"]} render={(m, i) => <tr key={i}><td>{m.start_tick}</td><td>{m.end_tick ?? "后续"}</td><td>{m.numerator}/{m.denominator_power <= 31 ? 2 ** m.denominator_power : `2^${m.denominator_power}`}</td><td>{m.bar_at_start ?? "未知"}</td><td>{origin(m.origin)}</td></tr>} />
    <p className="muted">小节与拍位是按拍号计算的名义网格；拍号变化开启新名义小节，不表示自动识别乐句。</p>
    <details><summary>原始拍号事件</summary><p className="muted">重复声明保留在这里，不会重置名义小节。</p>
      <PagedTable rows={r.source.time_signature_map} headings={["tick", "轨道 / 事件", "拍号", "点击参数"]} render={(m, i) => <tr key={i}><td>{m.location.tick}</td><td>{m.location.track} / {m.location.event}</td><td>{m.numerator}/2^{m.denominator_power}</td><td>{m.clocks_per_click} / {m.thirty_seconds_per_quarter}</td></tr>} />
    </details>
    <h3>诊断与不确定性</h3>
    <PagedTable rows={r.diagnostics} headings={["类别 / 代码", "说明"]} render={(d, i) => <tr key={i}><td>{d.category}<br /><code>{d.code}</code></td><td>{d.message}</td></tr>} />
  </section>;
});
