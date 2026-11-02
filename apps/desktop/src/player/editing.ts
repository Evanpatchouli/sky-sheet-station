import type { Score } from "./model.ts";

export type ScoreNote = Score["tracks"][number]["notes"][number];
export const GRID_OPTIONS = [
  { ticks: 960, label: "1/4 音符" },
  { ticks: 480, label: "1/8 音符" },
  { ticks: 240, label: "1/16 音符" },
  { ticks: 120, label: "1/32 音符" },
  { ticks: 60, label: "1/64 音符" },
];
export const snapTick = (tick: number, grid: number) =>
  Math.max(0, Math.round(tick / grid) * grid);
export const cellTick = (tick: number, grid: number) =>
  Math.max(0, Math.floor(tick / grid) * grid);
export const snapDuration = (duration: number, grid: number) =>
  Math.max(grid, snapTick(duration, grid));

// Editing constraints only. Original files are still read and validated by Rust.
export function placementError(
  score: Score,
  note: ScoreNote,
  replacing?: string,
): string | null {
  if (
    !Number.isSafeInteger(note.start_tick) ||
    note.start_tick < 0 ||
    !Number.isSafeInteger(note.duration_ticks) ||
    note.duration_ticks <= 0 ||
    !Number.isSafeInteger(note.start_tick + note.duration_ticks)
  )
    return "音符位置或长度超出可编辑范围。";
  if (!score.instrument.keys.includes(note.pitch))
    return "该音高不在当前乐器的键位中。";
  const end = note.start_tick + note.duration_ticks;
  if (
    score.tracks.some((track) =>
      track.notes.some(
        (other) =>
          other.id !== replacing &&
          other.pitch === note.pitch &&
          other.start_tick < end &&
          other.start_tick + other.duration_ticks > note.start_tick,
      ),
    )
  )
    return "此处已有相同音高的音符，请选择空白位置。";
  return null;
}

export function putNote(
  score: Score,
  note: ScoreNote,
  trackIndex: number,
  replacing?: string,
): Score {
  if (!score.tracks[trackIndex]) throw new Error("请选择有效的编辑轨道。");
  if (
    replacing &&
    !score.tracks[trackIndex].notes.some((n) => n.id === replacing)
  )
    throw new Error("待编辑音符已不存在。");
  const invalid = placementError(score, note, replacing);
  if (invalid) throw new Error(invalid);
  if (
    score.tracks.some((t) =>
      t.notes.some((n) => n.id === note.id && n.id !== replacing),
    )
  )
    throw new Error("音符标识重复。");
  return {
    ...score,
    tracks: score.tracks.map((track, index) =>
      index !== trackIndex
        ? track
        : {
            ...track,
            notes: [
              ...track.notes.filter((n) => n.id !== replacing),
              note,
            ].sort((a, b) => a.start_tick - b.start_tick),
          },
    ),
  };
}

export function removeNote(score: Score, id: string): Score {
  return {
    ...score,
    tracks: score.tracks.map((track) => ({
      ...track,
      notes: track.notes.filter((n) => n.id !== id),
    })),
  };
}

export function notesInCell(
  score: Score,
  trackIndex: number,
  tick: number,
  grid: number,
): ScoreNote[] {
  return (score.tracks[trackIndex]?.notes ?? []).filter(
    (note) =>
      note.start_tick < tick + grid &&
      note.start_tick + note.duration_ticks > tick,
  );
}

export function toggleNoteAtCell(
  score: Score,
  trackIndex: number,
  pitch: string,
  tick: number,
  grid: number,
  newId: string,
): { score: Score; selected: string | null } {
  // Imported notes can start between grid lines. Toggle the whole note occupying
  // this cell, in the edit track only; legacy overlaps are removed one at a time.
  const existing = notesInCell(score, trackIndex, tick, grid).find(
    (note) => note.pitch === pitch,
  );
  if (existing)
    return { score: removeNote(score, existing.id), selected: null };
  return {
    score: putNote(
      score,
      {
        id: newId,
        pitch,
        start_tick: tick,
        duration_ticks: grid,
      },
      trackIndex,
    ),
    selected: newId,
  };
}

export function resizeNote(
  note: ScoreNote,
  edge: "start" | "end",
  tick: number,
  grid: number,
): ScoreNote {
  if (edge === "end")
    return {
      ...note,
      duration_ticks: snapDuration(tick - note.start_tick, grid),
    };
  const end = note.start_tick + note.duration_ticks;
  const duration = Math.min(
    Math.floor(end / grid) * grid,
    snapDuration(end - Math.max(0, tick), grid),
  );
  // An imported sub-grid note near tick zero cannot grow to the left.
  if (duration < grid) return note;
  return { ...note, start_tick: end - duration, duration_ticks: duration };
}
