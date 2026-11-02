import { Select } from "./Select.tsx";
import { useEffect, useMemo, useRef, useState } from "react";
import type { PreparedScore, Score } from "../player/model.ts";
import { secondsAt, tickAt } from "../player/model.ts";
import {
  cellTick,
  GRID_OPTIONS,
  putNote,
  removeNote,
  snapDuration,
  notesInCell,
  toggleNoteAtCell,
} from "../player/editing.ts";
import type { ScoreNote } from "../player/editing.ts";
import type { PlayerEngine, PlaybackState } from "../player/engine.ts";
import { PianoRoll, TRACK_COLORS } from "./PianoRoll.tsx";
import { Keyboard } from "./Keyboard.tsx";

export function ScoreEditor({
  song,
  engine,
  state,
  disabled,
  dirty,
  onChange,
  transportPosition,
}: {
  song: PreparedScore | null;
  engine: PlayerEngine;
  state: PlaybackState;
  disabled: boolean;
  dirty: boolean;
  onChange: (score: Score) => void;
  transportPosition: { seconds: number } | null;
}) {
  const [grid, setGrid] = useState(240),
    [cursor, setCursor] = useState(0),
    [track, setTrack] = useState(0);
  const [zoom, setZoom] = useState(1),
    [follow, setFollow] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);
  const [clipboard, setClipboard] = useState<ScoreNote | null>(null);
  const [message, setMessage] = useState("");
  const undo = useRef<Score[]>([]),
    redo = useRef<Score[]>([]);
  const editor = useRef<HTMLDivElement>(null);
  const previousStatus = useRef(state.status);
  const lastTransportPosition = useRef(transportPosition);
  const seekingGrid = useRef(false);
  useEffect(() => {
    if (lastTransportPosition.current === transportPosition) return;
    lastTransportPosition.current = transportPosition;
    if (transportPosition && song)
      setCursor(cellTick(tickAt(transportPosition.seconds, song.score), grid));
  }, [transportPosition, song, grid]);
  useEffect(() => {
    if (
      previousStatus.current === "playing" &&
      state.status !== "playing" &&
      !seekingGrid.current &&
      song
    )
      setCursor(cellTick(tickAt(state.position, song.score), grid));
    previousStatus.current = state.status;
    seekingGrid.current = false;
  }, [state.status, state.position, song, grid]);
  const currentCursor =
    song && state.status === "playing"
      ? cellTick(tickAt(state.position, song.score), grid)
      : cursor;
  const occupied = useMemo(
    () =>
      song
        ? notesInCell(song.score, track, currentCursor, grid).map(
            (note) => note.pitch,
          )
        : [],
    [song, track, currentCursor, grid],
  );
  function commit(score: Score) {
    if (!song || disabled) return;
    undo.current = [...undo.current.slice(-99), song.score];
    redo.current = [];
    onChange(score);
    setMessage("");
  }
  function changeNote(note: ScoreNote, index: number, replacing?: string) {
    if (!song || disabled) return;
    try {
      commit(putNote(song.score, note, index, replacing));
      setSelected(note.id);
    } catch (e) {
      setMessage(e instanceof Error ? e.message : String(e));
    }
  }
  function copy(id: string) {
    const note = song?.score.tracks
      .flatMap((t) => t.notes)
      .find((n) => n.id === id);
    if (note) {
      setClipboard({ ...note });
      setMessage(`已复制 ${note.pitch}，将鼠标移到目标网格后粘贴。`);
    }
  }
  function paste(tick: number) {
    if (!clipboard) return;
    changeNote(
      {
        ...clipboard,
        id: crypto.randomUUID(),
        start_tick: cellTick(tick, grid),
        duration_ticks: snapDuration(clipboard.duration_ticks, grid),
      },
      track,
    );
  }
  function remove(id: string) {
    if (song) {
      commit(removeNote(song.score, id));
      setSelected(null);
    }
  }
  function history(direction: "undo" | "redo") {
    if (!song || disabled) return;
    const source = direction === "undo" ? undo : redo,
      target = direction === "undo" ? redo : undo;
    const score = source.current.pop();
    if (score) {
      target.current.push(song.score);
      onChange(score);
      setSelected(null);
      setMessage("");
    }
  }
  return (
    <div
      ref={editor}
      className="score-editor"
      onKeyDown={(e) => {
        if (
          disabled ||
          (e.target as HTMLElement).closest(
            "input, select, textarea, [role=menu], [role=combobox], [role=listbox]",
          )
        )
          return;
        const modifier = e.ctrlKey || e.metaKey;
        if (modifier && e.key.toLowerCase() === "c" && selected) {
          e.preventDefault();
          copy(selected);
        }
        if (e.key === "Delete" && selected) {
          e.preventDefault();
          remove(selected);
        }
        if (modifier && e.key.toLowerCase() === "z") {
          e.preventDefault();
          history(e.shiftKey ? "redo" : "undo");
        }
        if (modifier && e.key.toLowerCase() === "y") {
          e.preventDefault();
          history("redo");
        }
      }}
    >
      <div className="roll-toolbar">
        <div className="editor-controls">
          <label>
            编辑轨道{" "}
            <Select
              aria-label="编辑轨道"
              disabled={!song || disabled}
              value={track}
              onChange={(value) => setTrack(Number(value))}
            >
              {song?.score.tracks.map((t, i) => (
                <option key={t.id} value={i}>
                  {t.name}
                </option>
              ))}
            </Select>
          </label>
          <label>
            最小单元{" "}
            <Select
              aria-label="最小单元"
              value={grid}
              disabled={disabled}
              onChange={(value) => {
                setGrid(Number(value));
                setCursor(cellTick(cursor, Number(value)));
              }}
            >
              {GRID_OPTIONS.map((option) => (
                <option key={option.ticks} value={option.ticks}>
                  {option.label}
                </option>
              ))}
            </Select>
          </label>
          <button
            disabled={disabled || !undo.current.length}
            onClick={() => history("undo")}
            title="Ctrl+Z"
          >
            撤销
          </button>
          <button
            disabled={disabled || !redo.current.length}
            onClick={() => history("redo")}
            title="Ctrl+Y"
          >
            重做
          </button>
        </div>
        <div className="view-controls">
          <label>
            缩放{" "}
            <input
              aria-label="缩放"
              type="range"
              min="0.5"
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
        onSeek={(seconds) => {
          seekingGrid.current = true;
          engine.pause();
          engine.seek(seconds);
        }}
        grid={grid}
        cursor={cursor}
        selected={selected}
        disabled={disabled}
        onCursor={setCursor}
        onSelect={(id) => {
          setSelected(id);
          engine.pause();
          if (song && state.status === "playing") setCursor(currentCursor);
        }}
        onEdit={(note, index) => changeNote(note, index, note.id)}
        onCopy={copy}
        onDelete={remove}
        onPaste={paste}
        canPaste={!!clipboard}
      />
      <div className="editor-help" role="status" aria-live="polite">
        {message ||
          "点击网格定位 → 琴键添加 / 再点删除 · 拖动移动 · 两端 / 滚轮改长度 · Del 删除 · Ctrl+C / V 复制粘贴"}
      </div>
      <div className="score-status">
        <div className="legend">
          {song?.score.tracks.map((t, i) => (
            <span key={t.id}>
              <i
                style={{ background: TRACK_COLORS[i % TRACK_COLORS.length] }}
              />
              {t.name}
            </span>
          ))}
        </div>
        <span>
          {song
            ? `${song.score.timeline.tempo} BPM · ${song.score.timeline.time_signature.numerator}/${song.score.timeline.time_signature.denominator} · 插入位置 ${currentCursor} ticks`
            : "导入音乐后即可编辑"}
        </span>
        {selected && <span>已选中音符</span>}
      </div>
      <Keyboard
        song={song}
        active={state.keys}
        occupied={occupied}
        dirty={dirty}
        disabled={disabled}
        onToggle={(pitch) => {
          if (!song || disabled) return;
          const tick = currentCursor;
          setCursor(tick);
          engine.pause();
          engine.seek(secondsAt(tick, song.score));
          if (!song.score.tracks.length) {
            const score = {
              ...song.score,
              tracks: [
                {
                  id: crypto.randomUUID(),
                  name: "旋律",
                  role: "melody",
                  notes: [],
                },
              ],
            };
            commit(
              putNote(
                score,
                {
                  id: crypto.randomUUID(),
                  pitch,
                  start_tick: tick,
                  duration_ticks: grid,
                },
                0,
              ),
            );
          } else {
            try {
              const result = toggleNoteAtCell(
                song.score,
                track,
                pitch,
                tick,
                grid,
                crypto.randomUUID(),
              );
              commit(result.score);
              setSelected(result.selected);
            } catch (e) {
              setMessage(e instanceof Error ? e.message : String(e));
            }
          }
          editor.current?.querySelector<HTMLCanvasElement>("canvas")?.focus();
        }}
      />
    </div>
  );
}
