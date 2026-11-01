import type { PlayerEngine, PlaybackState } from "../player/engine.ts";
import type { PreparedScore } from "../player/model.ts";
import { formatTime } from "../player/model.ts";
import { Icon } from "./Icon.tsx";
export function Transport({
  song,
  engine,
  state,
}: {
  song: PreparedScore | null;
  engine: PlayerEngine;
  state: PlaybackState;
}) {
  const unavailable = !song || !!song.playbackError || song.notes.length === 0;
  const duration = song && !song.playbackError ? song.duration : 0;
  return (
    <footer className="transport" aria-label="播放控制">
      <div className="transport-buttons">
        <button
          className="primary play"
          aria-label="播放"
          disabled={unavailable || state.status === "playing"}
          onClick={() => void engine.play()}
        >
          <Icon name="play" />
        </button>
        <button
          aria-label="暂停"
          disabled={state.status !== "playing"}
          onClick={() => engine.pause()}
        >
          <Icon name="pause" />
        </button>
        <button
          aria-label="停止"
          disabled={!song}
          onClick={() => engine.stop()}
        >
          <Icon name="stop" />
        </button>
      </div>
      <div className="time" aria-label="播放时间">
        <strong>{formatTime(state.position)}</strong>
        <span>/</span>
        <span>{formatTime(duration)}</span>
      </div>
      <input
        className="progress"
        aria-label="播放进度"
        type="range"
        min="0"
        max={duration || 1}
        step="any"
        value={state.position}
        disabled={unavailable}
        onChange={(e) => engine.seek(Number(e.target.value))}
      />
      <select
        aria-label="播放速度"
        value={state.speed}
        onChange={(e) => engine.setSpeed(Number(e.target.value))}
      >
        {[0.5, 0.75, 1, 1.25, 1.5, 2].map((speed) => (
          <option key={speed} value={speed}>
            {speed}×
          </option>
        ))}
      </select>
      <button
        className={`loop ${state.loop ? "selected" : ""}`}
        aria-label="循环播放"
        aria-pressed={state.loop}
        onClick={() => engine.setLoop(!state.loop)}
      >
        <Icon name="loop" />
        <span>循环</span>
      </button>
    </footer>
  );
}
