import { memo, useMemo, useState } from "react";
import type { ArrangementReport as Report } from "../arrangement/model.ts";
const percent=(n:number|null)=>n===null?'不适用':`${(n*100).toFixed(1)}%`;
const reasons:Record<string,string>={midi_out_of_range:'超出 MIDI 范围',below_instrument_range:'低于乐器音域',above_instrument_range:'高于乐器音域',not_in_instrument_keys:'不在实际键位',accompaniment_simplified:'简易伴奏支撑音取舍',maximum_polyphony:'用户同时发声上限'};

export const ArrangementReport=memo(function ArrangementReport({report:r}:{report:Report}) {
  const [page,setPage]=useState(0),[lossOnly,setLossOnly]=useState(false);
  const dropped=useMemo(()=>r.note_mappings.filter(n=>!n.mapped),[r]);
  const shown=lossOnly?dropped:r.note_mappings;
  const pages=Math.max(1,Math.ceil(shown.length/20)),current=Math.min(page,pages-1);
  return <section className="music-analysis" aria-label="编曲报告">
    <p className="analysis-source">{r.source_file} · {r.target_instrument} · {r.arrangement_mode}</p>
    <div className="analysis-metrics">
      <div><strong>{r.transpose_semitones??'—'}</strong><span>全局移调（半音）</span></div>
      <div><strong>{r.kept}/{r.original_notes}</strong><span>保留音符 · {percent(r.retention)}</span></div>
      {r.role_retention.filter(x=>x.total>0).map(x=><div key={x.role}><strong>{x.kept}/{x.total}</strong><span>{x.role} · {percent(x.retention)}</span></div>)}
    </div>
    <p>舍弃 {r.dropped} · 八度调整 {r.playability.octave_adjusted_notes} · 最大同时发声 {r.playability.maximum_simultaneous_notes} · 旋律音程变化 {r.playability.melody_interval_changes}</p>
    <p className="muted">角色来自未校准启发式。Unknown 不自动算作伴奏；保留率分母为该角色全部源音符，零分母不适用。</p>
    <details><summary>选择依据与诊断</summary><p>{r.melody.reason}</p>{r.diagnostics.map((d,i)=><p key={i}>{d}</p>)}</details>
    <div className="analysis-table-scroll"><table><thead><tr><th>源轨道（0 起）</th><th>角色</th><th>源 → 目标 MIDI 音域</th><th>保留 / 舍弃 / 八度调整</th></tr></thead><tbody>
      {r.tracks.map(t=><tr key={t.source_track}><td>{t.source_track} · {t.name??'未命名'}</td><td>{t.phase3_role.role}</td><td>{t.source_lowest_midi??'—'}–{t.source_highest_midi??'—'} → {t.final_lowest_midi??'—'}–{t.final_highest_midi??'—'}</td><td>{t.kept} / {t.dropped} / {t.adjusted}</td></tr>)}
    </tbody></table></div>
    <details><summary>可演奏性、和声与复音诊断</summary>
      <p>同键重叠 {r.playability.same_key_overlap_events} · 跨轨同键 {r.playability.cross_track_key_conflicts} · 低音/旋律交叉 {r.playability.bass_melody_crossing_events} · 新增旋律大跳 {r.playability.unnecessary_large_leaps}</p>
      <p>伴奏起音时刻保留 {r.playability.retained_accompaniment_onsets}/{r.playability.source_accompaniment_onsets}。结构检查不能证明音乐性或演奏便利，需要人工试听。</p>
      <p>同时起音组 {r.onset_groups.length} · 和弦候选 {r.onset_groups.filter(g=>g.chord_candidate).length} · 音程配置变化组 {r.onset_groups.filter(g=>g.exact_intervals_changed).length}</p>
      {r.onset_groups.filter(g=>g.chord_candidate).slice(0,20).map(g=><p key={g.start_tick}>Tick {g.start_tick}: {g.chord_candidate}</p>)}
      {r.conflicts.slice(0,30).map((c,i)=><p key={i}>Tick {c.start_tick} · {c.reason} · 保留 {c.kept}/{c.source_count} · {c.source_note_ids.join(', ')} · {c.resolution}</p>)}
      <p className="muted">此处最多展示 20 个和弦候选、30 条冲突，完整记录保存在独立报告中。</p>
    </details>
    <details open><summary>逐音适配明细（全部 {r.note_mappings.length} / 舍弃 {dropped.length}）</summary>
      <label><input type="checkbox" checked={lossOnly} onChange={e=>{setLossOnly(e.target.checked);setPage(0);}}/>仅显示舍弃音符</label>
      <div className="analysis-table-scroll"><table><thead><tr><th>源轨道 / 音符</th><th>MIDI 源 → 全局 → 最终</th><th>源 tick / 时长</th><th>投影 tick / 时长</th><th>状态与依据</th></tr></thead><tbody>
        {shown.slice(current*20,current*20+20).map(n=><tr key={n.source_note_id}><td>{n.source_track} / {n.source_note_id}</td><td>{n.source_midi} → {n.global_transposed_midi} → {n.target_midi}<br/>八度 {n.voice_octave_shift} · 键位 {n.key_index===null?'—':n.key_index+1}</td><td>{n.source_start_tick} / {n.source_duration_ticks}</td><td>{n.projected_start_tick} / {n.projected_duration_ticks}</td><td>{n.mapped?'保留':reasons[n.drop_reason??'']??n.drop_reason}<details><summary>映射依据</summary>{n.mapping_reason}<p>前/后音程变化：{n.previous_interval_change??'—'} / {n.next_interval_change??'—'}；重要性 {n.importance}（启发式）</p>{n.importance_reasons.join('; ')}</details></td></tr>)}
      </tbody></table>{!shown.length&&<p>没有匹配音符。</p>}</div>
      {pages>1&&<div className="analysis-pagination"><button disabled={current===0} onClick={()=>setPage(current-1)}>上一页</button><span>{current+1}/{pages}</span><button disabled={current+1===pages} onClick={()=>setPage(current+1)}>下一页</button></div>}
    </details>
    <details><summary>全局移调候选（{r.candidates.length}，最终 DP 前的保留上界）</summary><div className="analysis-table-scroll"><table><thead><tr><th>半音</th><th>旋律</th><th>全部</th><th>伴奏（含低音）</th></tr></thead><tbody>{r.candidates.map(c=><tr key={c.transpose_semitones}><td>{c.transpose_semitones}{c.selected?' ✓':''}</td><td>{c.melody_kept??'未知'}</td><td>{c.total_kept}</td><td>{c.accompaniment_kept}</td></tr>)}</tbody></table></div></details>
    <p>{r.score_validation&&r.time_preserved&&r.pitch_mapping_valid&&r.playability.arrangement_constraints_satisfied?'Score、投影时间、键位和用户复音约束验证通过。':'尚未通过全部输出检查。'}</p>
    {r.score_file?<p className="analysis-source">Score：{r.score_file}<br/>报告：{r.report_file}</p>:<p>当前是未保存预览。加载试听后可返回此面板保存。</p>}
  </section>;
});
