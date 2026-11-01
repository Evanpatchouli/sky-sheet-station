import type { PreparedScore } from "../player/model.ts";
export function Keyboard({
  song,
  active,
}: {
  song: PreparedScore | null;
  active: number[];
}) {
  return (
    <section className="instrument" aria-label="乐器键盘">
      <div className="instrument-heading">
        <h2>
          {song?.score.instrument.id === "sky-15"
            ? "Sky 15"
            : (song?.score.instrument.name ?? "你的乐器")}
        </h2>
        <p>本地合成试听</p>
        <span>
          {song
            ? `${song.score.instrument.keys.length} 键 · ${song.score.instrument.playback_mode}`
            : "随乐谱载入键位"}
        </span>
      </div>
      <div
        className="keyboard"
        role="list"
        aria-label="按乐器原始顺序排列的键位"
      >
        {song ? (
          song.score.instrument.keys.map((pitch, index) => (
            <div
              key={pitch}
              role="listitem"
              aria-label={`按键 ${index + 1} ${pitch}${active.includes(index) ? " 发声中" : ""}`}
              className={`sky-key ${active.includes(index) ? "active" : ""}`}
            >
              <small>{String(index + 1).padStart(2, "0")}</small>
              <span
                className={`key-shape ${index % 2 ? "diamond" : "circle"}`}
              />
              <b>{pitch}</b>
            </div>
          ))
        ) : (
          <div className="keyboard-placeholder">
            键位来自乐谱中的 Instrument
            <br />
            <span>加载后在这里同步显示</span>
          </div>
        )}
      </div>
      <div className="instrument-note">
        <span className="status-dot" />
        {song ? "已通过 CLI 验证" : "等待 Score JSON"}
        <p>音频转换尚未接通</p>
        <span>试听音色不模拟游戏原声</span>
      </div>
    </section>
  );
}
