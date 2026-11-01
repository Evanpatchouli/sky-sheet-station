import type { ImportReport as Report } from "../player/model.ts";
import { useMemo } from "react";

export function ImportReport({ report }: { report: Report }) {
  const diagnostics = useMemo(
    () => [...report.diagnostics].sort((a, b) =>
      (a.severity === "error" ? 0 : a.severity === "warning" ? 1 : 2) -
      (b.severity === "error" ? 0 : b.severity === "warning" ? 1 : 2)),
    [report],
  );
  const failed = diagnostics.some((d) => d.severity === "error");
  return (
    <div className="import-report">
      {failed && <p>此次转换已拒绝，未生成新乐谱。错误优先列在下方。</p>}
      <p>
        {report.track_count} 轨 · {report.note_count} 个已配对音符 · 源 PPQ {report.source_ppq ?? "未知"}
      </p>
      <p>
        目标参考速度：{report.reference_tempo ?? "未确定"} BPM。
        时间采用绝对起止点量化，保留前置静音。
      </p>
      <p>
        源速度事件 {report.tempos.length} 个，拍号事件 {report.meters.length} 个。
        后续速度变化按时间展开；后续拍号仅保留在报告。
      </p>
      <details>
        <summary>源速度与拍号事件</summary>
        <pre>{JSON.stringify({ tempos: report.tempos, meters: report.meters }, null, 2)}</pre>
      </details>
      <ul className="diagnostic-list">
        {diagnostics.map((d, i) => (
          <li key={i}>
            <strong>
              {d.severity === "error" ? "错误" : d.severity === "warning" ? "提示" : "信息"} · {d.code}
            </strong>
            <p>{d.message}</p>
            {d.code === "unsupported_pitch" && (
              <p>此音高不在 sky-15 的 15 键集合中；本阶段不会自动移调或删除音符。</p>
            )}
            {d.track !== null && (
              <small>源轨 {d.track + 1} · 事件 {(d.event ?? 0) + 1} · tick {d.source_tick}</small>
            )}
          </li>
        ))}
      </ul>
      <details>
        <summary>完整导入报告（包含源音符）</summary>
        <pre>{JSON.stringify(report, null, 2)}</pre>
      </details>
    </div>
  );
}
