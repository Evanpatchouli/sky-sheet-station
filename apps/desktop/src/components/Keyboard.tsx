import type { PreparedScore } from "../player/model.ts";
export function Keyboard({
  song,
  active,
  onToggle,
  occupied,
  dirty,
  disabled,
}: {
  song: PreparedScore | null;
  active: number[];
  onToggle: (pitch: string) => void;
  occupied: string[];
  dirty: boolean;
  disabled: boolean;
}) {
  return (
    <section className="instrument" aria-label="乐器键盘">
      <div className="instrument-heading">
        <h2>
          {song?.score.instrument.id === "sky-15"
            ? "Sky 15"
            : (song?.score.instrument.name ?? "你的乐器")}
        </h2>
        <p>点击添加，再点删除</p>
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
            <button
              key={pitch}
              disabled={disabled}
              onClick={() => onToggle(pitch)}
              title={
                occupied.includes(pitch)
                  ? `删除当前轨道这一格的 ${pitch} 音符`
                  : `在当前轨道这一格添加 ${pitch}`
              }
              aria-pressed={occupied.includes(pitch)}
              aria-label={`按键 ${index + 1} ${pitch}${active.includes(index) ? " 发声中" : ""}`}
              className={`sky-key ${active.includes(index) ? "active" : ""} ${occupied.includes(pitch) ? "occupied" : ""}`}
            >
              <small>{String(index + 1).padStart(2, "0")}</small>
              <span
                className={`key-shape ${index % 2 ? "diamond" : "circle"}`}
              />
              <b>{pitch}</b>
            </button>
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
        {song ? (dirty ? "有未保存的修改" : "已通过 CLI 验证") : "等待乐谱"}
        <p>亮框表示当前轨道这一格已有音符</p>
        <span>试听音色不模拟游戏原声</span>
      </div>
    </section>
  );
}
