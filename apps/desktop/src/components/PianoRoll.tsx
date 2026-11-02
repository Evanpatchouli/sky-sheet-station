import { useEffect, useMemo, useRef, useState } from "react";
import type { PreparedScore } from "../player/model.ts";
import { formatTime, meterTicks, secondsAt, tickAt } from "../player/model.ts";
import {
  cellTick,
  placementError,
  resizeNote,
  snapDuration,
  snapTick,
} from "../player/editing.ts";
import type { ScoreNote } from "../player/editing.ts";
import type { PlayerNote } from "../player/model.ts";
import { ContextMenu } from "./ContextMenu.tsx";
export const TRACK_COLORS = [
  "#c6cfdf",
  "#38b9d6",
  "#bbabdf",
  "#e5b478",
  "#8bcaaf",
  "#dc91a7",
];
const LABEL = 60,
  RULER = 42;

export function PianoRoll({
  song,
  position,
  playing,
  zoom,
  follow,
  onSeek,
  grid,
  cursor,
  selected,
  onSelect,
  onCursor,
  onEdit,
  onCopy,
  onDelete,
  onPaste,
  canPaste,
  disabled,
}: {
  song: PreparedScore | null;
  position: number;
  playing: boolean;
  zoom: number;
  follow: boolean;
  onSeek: (seconds: number) => void;
  grid: number;
  cursor: number;
  selected: string | null;
  disabled: boolean;
  onSelect: (id: string | null) => void;
  onCursor: (tick: number) => void;
  onEdit: (note: ScoreNote, track: number) => void;
  onCopy: (id: string) => void;
  onDelete: (id: string) => void;
  onPaste: (tick: number) => void;
  canPaste: boolean;
}) {
  const canvas = useRef<HTMLCanvasElement>(null),
    viewport = useRef<HTMLDivElement>(null),
    scroll = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 1000, height: 340 });
  const [offset, setOffset] = useState(0);
  const gesture = useRef<{
    note: ScoreNote;
    track: number;
    originX: number;
    originTick: number;
    mode: "move" | "start" | "end";
    changed: boolean;
  } | null>(null);
  const seeking = useRef(false);
  const [preview, setPreview] = useState<{
    note: ScoreNote;
    track: number;
    invalid: boolean;
  } | null>(null);
  const [mouseCursor, setMouseCursor] = useState("crosshair");
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    id: string | null;
    tick: number;
  } | null>(null);
  const hoverTick = useRef<number | null>(null);
  const ppq = song?.score.timeline.ppq ?? 960;
  const pixelsPerTick = (zoom * 150) / ppq;
  const visibleTicks = Math.max(1, size.width - LABEL) / pixelsPerTick;
  const extent = Math.min(
    Number.MAX_SAFE_INTEGER,
    Math.max((song?.endTick ?? 0) + ppq * 16, cursor + ppq * 4, visibleTicks),
  );
  const maxOffset = Math.max(0, extent - visibleTicks);
  const scrollWidth = Math.min(
    4_000_000,
    Math.max(size.width, extent * pixelsPerTick + LABEL),
  );
  const safeOffset = Math.min(offset, maxOffset);
  const rows = useMemo(
    () =>
      song
        ? song.score.instrument.keys
            .map((pitch, index) => ({
              pitch,
              index,
              midi: song.midi_keys[index],
            }))
            .sort((a, b) => b.midi - a.midi || a.index - b.index)
        : [],
    [song],
  );
  const rowOf = useMemo(
    () => new Map(rows.map((row, i) => [row.index, i])),
    [rows],
  );
  const rowHeight = (size.height - RULER) / Math.max(rows.length, 15);
  const currentTick =
    song && playing && !song.playbackError
      ? tickAt(position, song.score)
      : cursor;

  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      }),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    setOffset(0);
    if (scroll.current) scroll.current.scrollLeft = 0;
  }, [song?.score.metadata.id, song?.source]);
  useEffect(() => {
    if (!playing || !follow || !scroll.current) return;
    if (
      currentTick < safeOffset ||
      currentTick > safeOffset + visibleTicks * 0.85
    ) {
      const next = Math.max(
        0,
        Math.min(maxOffset, currentTick - visibleTicks * 0.15),
      );
      scroll.current.scrollLeft = maxOffset
        ? (next / maxOffset) * (scrollWidth - size.width)
        : 0;
      setOffset(next);
    }
  }, [
    currentTick,
    playing,
    follow,
    safeOffset,
    visibleTicks,
    maxOffset,
    scrollWidth,
    size.width,
  ]);

  useEffect(() => {
    const element = canvas.current,
      ctx = element?.getContext("2d");
    if (!element || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    element.width = size.width * dpr;
    element.height = size.height * dpr;
    ctx.scale(dpr, dpr);
    ctx.fillStyle = "#131920";
    ctx.fillRect(0, 0, size.width, size.height);
    ctx.font = '11px "Segoe UI", sans-serif';
    for (let i = 0; i < Math.max(rows.length, 15); i++) {
      const y = RULER + i * rowHeight;
      ctx.fillStyle = i % 2 ? "#161d25" : "#131920";
      ctx.fillRect(LABEL, y, size.width - LABEL, rowHeight);
      ctx.strokeStyle = "#242d37";
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(size.width, y);
      ctx.stroke();
      if (rows[i]) {
        ctx.fillStyle = "#a1adbd";
        ctx.fillText(rows[i].pitch, 16, y + rowHeight / 2 + 4);
      }
    }
    if (!song) return;
    const { beat, bar } = meterTicks(song.score);
    // O(viewport width) ruler work, including tiny meters and enormous end ticks.
    const gridStep = grid;
    ctx.save();
    ctx.beginPath();
    ctx.rect(LABEL, 0, size.width - LABEL, size.height);
    ctx.clip();
    for (
      let tick = Math.floor(safeOffset / gridStep) * gridStep;
      tick <= safeOffset + visibleTicks;
      tick += gridStep
    ) {
      const x = LABEL + (tick - safeOffset) * pixelsPerTick;
      ctx.strokeStyle =
        tick % bar === 0
          ? "#647080"
          : tick % beat === 0
            ? "#3d4b5b"
            : "#263240";
      ctx.beginPath();
      ctx.moveTo(x, RULER - 5);
      ctx.lineTo(x, size.height);
      ctx.stroke();
    }
    const rulerStep = bar * Math.max(1, Math.ceil(100 / (bar * pixelsPerTick)));
    for (
      let tick = Math.floor(safeOffset / rulerStep) * rulerStep;
      tick <= safeOffset + visibleTicks;
      tick += rulerStep
    ) {
      const x = LABEL + (tick - safeOffset) * pixelsPerTick;
      const seconds = secondsAt(tick, song.score);
      ctx.fillStyle = "#a7b3c4";
      ctx.fillText(
        song.playbackError
          ? `${tick} tick`
          : `${formatTime(seconds)}.${String(Math.floor((seconds % 1) * 1000)).padStart(3, "0")}`,
        x + 5,
        15,
      );
      ctx.fillStyle = "#6e8298";
      ctx.fillText(`${Math.floor(tick / bar) + 1}`, x + 5, 32);
    }
    const visible = song.index.between(safeOffset, safeOffset + visibleTicks);
    if (preview) {
      const key = song.score.instrument.keys.indexOf(preview.note.pitch);
      visible.push({
        id: "__preview__",
        track: preview.track,
        key,
        startTick: preview.note.start_tick,
        endTick: preview.note.start_tick + preview.note.duration_ticks,
        start: 0,
        end: 0,
        midi: song.midi_keys[key],
      });
    }
    for (const note of visible) {
      const x = LABEL + (note.startTick - safeOffset) * pixelsPerTick;
      const endX = LABEL + (note.endTick - safeOffset) * pixelsPerTick;
      const left = Math.max(LABEL, x),
        right = Math.min(size.width, endX);
      if (right < LABEL || left > size.width || right < left) continue;
      const laneCount = Math.min(3, song.score.tracks.length);
      const noteHeight = Math.max(2, (rowHeight - 4) / laneCount);
      const y =
        RULER +
        rowOf.get(note.key)! * rowHeight +
        2 +
        (note.track % laneCount) * noteHeight;
      ctx.fillStyle = TRACK_COLORS[note.track % TRACK_COLORS.length];
      const ghost = note.id === "__preview__";
      ctx.globalAlpha = ghost
        ? preview?.invalid
          ? 0.22
          : 0.6
        : preview?.note.id === note.id
          ? 0.2
          : 1;
      if (note.id === selected || ghost)
        ctx.fillStyle = ghost && preview?.invalid ? "#f5a49e" : "#53d2e9";
      ctx.beginPath();
      ctx.roundRect(
        left,
        y,
        Math.max(1, right - left),
        Math.max(2, noteHeight - 1),
        2,
      );
      ctx.fill();
      if (note.id === selected || ghost) {
        ctx.strokeStyle = ghost && preview?.invalid ? "#f5a49e" : "#c3f7ff";
        ctx.stroke();
        if (!ghost) {
          ctx.fillStyle = "#e1fbff";
          if (x >= LABEL)
            ctx.fillRect(x + 1, y + 1, 3, Math.max(2, noteHeight - 3));
          if (endX <= size.width)
            ctx.fillRect(endX - 4, y + 1, 3, Math.max(2, noteHeight - 3));
        }
      }
      ctx.globalAlpha = 1;
    }
    ctx.restore();
  }, [
    song,
    size,
    rows,
    rowOf,
    rowHeight,
    pixelsPerTick,
    safeOffset,
    visibleTicks,
    grid,
    selected,
    preview,
  ]);

  const seek = (clientX: number) => {
    if (!song || !canvas.current) return;
    const x = clientX - canvas.current.getBoundingClientRect().left;
    const tick = cellTick(safeOffset + (x - LABEL) / pixelsPerTick, grid);
    onCursor(tick);
    if (!song.playbackError) onSeek(secondsAt(tick, song.score));
  };
  function point(clientX: number, clientY: number) {
    const rect = canvas.current!.getBoundingClientRect();
    const x = clientX - rect.left,
      y = clientY - rect.top;
    const tick = Math.max(0, safeOffset + (x - LABEL) / pixelsPerTick);
    const row = rows[Math.floor((y - RULER) / rowHeight)];
    return {
      x,
      y,
      tick,
      row,
      inside: x >= LABEL && x <= size.width && y >= RULER && y < size.height,
    };
  }
  function hit(clientX: number, clientY: number): PlayerNote | undefined {
    if (!song) return;
    const p = point(clientX, clientY);
    if (!p.inside) return;
    const lanes = Math.min(3, song.score.tracks.length);
    const height = Math.max(2, (rowHeight - 4) / lanes);
    return song.index
      .between(p.tick, p.tick)
      .reverse()
      .find((n) => {
        const top =
          RULER +
          rowOf.get(n.key)! * rowHeight +
          2 +
          (n.track % lanes) * height;
        return n.key === p.row?.index && p.y >= top && p.y <= top + height;
      });
  }
  function original(note: PlayerNote) {
    return song!.score.tracks[note.track].notes.find((n) => n.id === note.id)!;
  }
  function modeFor(note: PlayerNote, x: number) {
    const start = LABEL + (note.startTick - safeOffset) * pixelsPerTick;
    const end = LABEL + (note.endTick - safeOffset) * pixelsPerTick;
    const handle = Math.min(6, (end - start) / 3);
    return x - start < handle ? "start" : end - x < handle ? "end" : "move";
  }
  // A non-passive listener prevents wheel resizing from scrolling the page.
  const wheelAction = useRef<(event: WheelEvent) => void>(() => {});
  wheelAction.current = (e) => {
    if (disabled || !song || !selected || e.ctrlKey || gesture.current) return;
    const target = hit(e.clientX, e.clientY);
    if (!target || target.id !== selected || !e.deltaY) return;
    e.preventDefault();
    const note = original(target);
    onEdit(
      {
        ...note,
        duration_ticks: Math.max(
          grid,
          snapDuration(note.duration_ticks, grid) +
            (e.deltaY < 0 ? grid : -grid),
        ),
      },
      target.track,
    );
  };
  useEffect(() => {
    const element = canvas.current!;
    const handle = (event: WheelEvent) => wheelAction.current(event);
    element.addEventListener("wheel", handle, { passive: false });
    return () => element.removeEventListener("wheel", handle);
  }, []);
  function closeMenu() {
    setMenu(null);
    canvas.current?.focus();
  }
  const pointerX = LABEL + (currentTick - safeOffset) * pixelsPerTick;
  return (
    <section className="roll" aria-label="乐谱钢琴卷帘">
      <div className="roll-viewport" ref={viewport}>
        <canvas
          ref={canvas}
          tabIndex={0}
          style={{ cursor: mouseCursor }}
          onKeyDown={(e) => {
            if (disabled) return;
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "v") {
              e.preventDefault();
              onPaste(hoverTick.current ?? cursor);
            }
            if (e.key === "Escape") {
              gesture.current = null;
              setPreview(null);
              setMouseCursor("crosshair");
            }
          }}
          aria-label={
            song
              ? `${song.notes.length} 个音符；点击空白定位，选择音符后拖动或调整长度`
              : "尚未加载乐谱"
          }
          onPointerDown={(e) => {
            if (disabled || !song || e.button !== 0) return;
            e.currentTarget.focus();
            setMenu(null);
            const p = point(e.clientX, e.clientY);
            if (p.x < LABEL) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            const note = hit(e.clientX, e.clientY);
            if (note) {
              onSelect(note.id);
              gesture.current = {
                note: original(note),
                track: note.track,
                originX: e.clientX,
                originTick: p.tick,
                mode: modeFor(note, p.x),
                changed: false,
              };
            } else {
              onSelect(null);
              seeking.current = p.y < RULER;
              seek(e.clientX);
            }
          }}
          onPointerMove={(e) => {
            if (disabled || !song) return;
            const p = point(e.clientX, e.clientY);
            hoverTick.current = p.inside ? cellTick(p.tick, grid) : null;
            const drag = gesture.current;
            if (drag) {
              if (
                !drag.changed &&
                Math.abs(e.clientX - drag.originX) < 3 &&
                p.row?.pitch === drag.note.pitch
              )
                return;
              drag.changed = true;
              const note =
                drag.mode === "move"
                  ? {
                      ...drag.note,
                      start_tick: snapTick(
                        drag.note.start_tick + p.tick - drag.originTick,
                        grid,
                      ),
                      duration_ticks: snapDuration(
                        drag.note.duration_ticks,
                        grid,
                      ),
                      pitch: p.row?.pitch ?? drag.note.pitch,
                    }
                  : resizeNote(drag.note, drag.mode, p.tick, grid);
              const invalid =
                !p.inside || !!placementError(song.score, note, drag.note.id);
              setPreview({ note, track: drag.track, invalid });
              setMouseCursor(
                invalid
                  ? "not-allowed"
                  : drag.mode === "move"
                    ? "grabbing"
                    : "ew-resize",
              );
            } else if (seeking.current) seek(e.clientX);
            else {
              const note = hit(e.clientX, e.clientY);
              setMouseCursor(
                note
                  ? modeFor(note, p.x) === "move"
                    ? "grab"
                    : "ew-resize"
                  : "crosshair",
              );
            }
          }}
          onPointerUp={() => {
            if (gesture.current?.changed && preview && !preview.invalid)
              onEdit(preview.note, preview.track);
            gesture.current = null;
            seeking.current = false;
            setPreview(null);
            setMouseCursor("crosshair");
          }}
          onPointerCancel={() => {
            gesture.current = null;
            seeking.current = false;
            setPreview(null);
            setMouseCursor("crosshair");
          }}
          onLostPointerCapture={() => {
            gesture.current = null;
            seeking.current = false;
            setPreview(null);
          }}
          onPointerLeave={() => {
            hoverTick.current = null;
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            if (!song || disabled) return;
            const p = point(e.clientX, e.clientY);
            if (!p.inside) return;
            const note = hit(e.clientX, e.clientY);
            onSelect(note?.id ?? null);
            setMenu({
              x: e.clientX,
              y: e.clientY,
              id: note?.id ?? null,
              tick: cellTick(p.tick, grid),
            });
          }}
        />
        {song && pointerX >= LABEL && pointerX <= size.width && (
          <div className="playhead" style={{ left: pointerX }} />
        )}
        {!song && (
          <div className="roll-empty">
            <strong>让乐谱在这里展开</strong>
            <span>导入音频或 MIDI 生成乐谱，也可以打开已有乐谱。</span>
          </div>
        )}
      </div>
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onClose={closeMenu}
          items={
            menu.id
              ? [
                  {
                    label: "复制",
                    icon: "copy",
                    shortcut: "Ctrl+C",
                    action: () => onCopy(menu.id!),
                  },
                  {
                    label: "删除",
                    icon: "delete",
                    shortcut: "Del",
                    action: () => onDelete(menu.id!),
                  },
                ]
              : [
                  {
                    label: "粘贴",
                    icon: "paste",
                    shortcut: "Ctrl+V",
                    disabled: !canPaste,
                    action: () => onPaste(menu.tick),
                  },
                ]
          }
        />
      )}
      <div
        className="roll-scroll"
        ref={scroll}
        aria-label="水平滚动乐谱"
        tabIndex={0}
        onScroll={(e) => {
          const available = scrollWidth - size.width;
          setOffset(
            available > 0
              ? (e.currentTarget.scrollLeft / available) * maxOffset
              : 0,
          );
        }}
      >
        <div style={{ width: scrollWidth, height: 1 }} />
      </div>
    </section>
  );
}
