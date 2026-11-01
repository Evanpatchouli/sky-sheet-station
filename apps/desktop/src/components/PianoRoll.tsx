import { useEffect, useMemo, useRef, useState } from "react";
import type { PreparedScore } from "../player/model.ts";
import { formatTime, meterTicks, secondsAt, tickAt } from "../player/model.ts";
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
}: {
  song: PreparedScore | null;
  position: number;
  playing: boolean;
  zoom: number;
  follow: boolean;
  onSeek: (seconds: number) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null),
    viewport = useRef<HTMLDivElement>(null),
    scroll = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 1000, height: 340 });
  const [offset, setOffset] = useState(0);
  const dragging = useRef(false);
  const ppq = song?.score.timeline.ppq ?? 960;
  const pixelsPerTick = (zoom * 150) / ppq;
  const visibleTicks = Math.max(1, size.width - LABEL) / pixelsPerTick;
  const extent = Math.max(song?.endTick ?? 0, visibleTicks);
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
    song && !song.playbackError ? tickAt(position, song.score) : 0;

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
  }, [song]);
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
    const gridStep = beat * Math.max(1, Math.ceil(24 / (beat * pixelsPerTick)));
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
      ctx.strokeStyle = tick % bar === 0 ? "#47515d" : "#28333f";
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
    for (const note of song.index.between(
      safeOffset,
      safeOffset + visibleTicks,
    )) {
      const x = LABEL + (note.startTick - safeOffset) * pixelsPerTick;
      const endX = LABEL + (note.endTick - safeOffset) * pixelsPerTick;
      const left = Math.max(LABEL, x),
        right = Math.min(size.width, endX);
      const laneCount = Math.min(3, song.score.tracks.length);
      const noteHeight = Math.max(2, (rowHeight - 4) / laneCount);
      const y =
        RULER +
        rowOf.get(note.key)! * rowHeight +
        2 +
        (note.track % laneCount) * noteHeight;
      ctx.fillStyle = TRACK_COLORS[note.track % TRACK_COLORS.length];
      ctx.beginPath();
      ctx.roundRect(
        left,
        y,
        Math.max(1, right - left),
        Math.max(2, noteHeight - 1),
        2,
      );
      ctx.fill();
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
  ]);

  const seek = (clientX: number) => {
    if (!song || song.playbackError || !canvas.current) return;
    const x = clientX - canvas.current.getBoundingClientRect().left;
    onSeek(
      secondsAt(
        Math.max(0, safeOffset + (x - LABEL) / pixelsPerTick),
        song.score,
      ),
    );
  };
  const pointerX = LABEL + (currentTick - safeOffset) * pixelsPerTick;
  return (
    <section className="roll" aria-label="乐谱钢琴卷帘">
      <div className="roll-viewport" ref={viewport}>
        <canvas
          ref={canvas}
          aria-label={
            song
              ? `${song.notes.length} 个音符；点击或拖动定位`
              : "尚未加载乐谱"
          }
          onPointerDown={(e) => {
            dragging.current = true;
            e.currentTarget.setPointerCapture(e.pointerId);
            seek(e.clientX);
          }}
          onPointerMove={(e) => {
            if (dragging.current) seek(e.clientX);
          }}
          onPointerUp={() => {
            dragging.current = false;
          }}
          onPointerCancel={() => {
            dragging.current = false;
          }}
        />
        {song && pointerX >= LABEL && pointerX <= size.width && (
          <div className="playhead" style={{ left: pointerX }} />
        )}
        {!song && (
          <div className="roll-empty">
            <strong>让乐谱在这里展开</strong>
            <span>打开 Score JSON，查看音符与键位的每一次呼应。</span>
          </div>
        )}
      </div>
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
