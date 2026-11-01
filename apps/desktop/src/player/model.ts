// IPC view of the Rust-validated Score, not a second input parser or validator.
export interface Score {
  format_version: string;
  metadata: { id: string; name: string };
  instrument: {
    id: string;
    name: string;
    playback_mode: "trigger" | "sustain";
    keys: string[];
  };
  timeline: {
    ppq: number;
    tempo: number;
    time_signature: { numerator: number; denominator: number };
  };
  tracks: {
    id: string;
    name: string;
    role: string;
    notes: {
      id: string;
      pitch: string;
      start_tick: number;
      duration_ticks: number;
    }[];
  }[];
  text: string | null;
}
export interface LoadedScore {
  score: Score;
  midi_keys: number[];
  source: string;
}
export interface PlayerNote {
  id: string;
  key: number;
  track: number;
  startTick: number;
  endTick: number;
  start: number;
  end: number;
  midi: number;
}
export interface PreparedScore extends LoadedScore {
  notes: PlayerNote[];
  duration: number;
  endTick: number;
  playbackError: string | null;
  index: NoteIndex;
}

// An interval tree supports seek/highlight and viewport culling without one DOM
// component per note, and without scanning the whole song on every audio tick.
interface Branch {
  note: PlayerNote;
  maxEnd: number;
  left: Branch | null;
  right: Branch | null;
}
export class NoteIndex {
  private root: Branch | null;
  constructor(notes: PlayerNote[]) {
    const build = (lo: number, hi: number): Branch | null => {
      if (lo >= hi) return null;
      const mid = (lo + hi) >>> 1,
        note = notes[mid];
      const left = build(lo, mid),
        right = build(mid + 1, hi);
      return {
        note,
        left,
        right,
        maxEnd: Math.max(note.endTick, left?.maxEnd ?? 0, right?.maxEnd ?? 0),
      };
    };
    this.root = build(0, notes.length);
  }
  between(start: number, end: number): PlayerNote[] {
    const result: PlayerNote[] = [];
    const visit = (branch: Branch | null) => {
      if (!branch || branch.maxEnd <= start) return;
      visit(branch.left);
      if (branch.note.startTick <= end && branch.note.endTick > start)
        result.push(branch.note);
      if (branch.note.startTick <= end) visit(branch.right);
    };
    visit(this.root);
    return result;
  }
}

export function secondsAt(ticks: number, score: Score): number {
  return ((ticks / score.timeline.ppq) * 60) / score.timeline.tempo;
}
export function tickAt(seconds: number, score: Score): number {
  return (seconds / 60) * score.timeline.tempo * score.timeline.ppq;
}
export function meterTicks(score: Score) {
  const beat =
    (score.timeline.ppq * 4) / score.timeline.time_signature.denominator;
  return { beat, bar: beat * score.timeline.time_signature.numerator };
}
export function prepareScore(loaded: LoadedScore): PreparedScore {
  const keys = new Map(
    loaded.score.instrument.keys.map((pitch, key) => [pitch, key]),
  );
  const notes = loaded.score.tracks
    .flatMap((track, trackIndex) =>
      track.notes.map((note) => {
        const key = keys.get(note.pitch)!;
        return {
          id: note.id,
          key,
          track: trackIndex,
          midi: loaded.midi_keys[key],
          startTick: note.start_tick,
          endTick: note.start_tick + note.duration_ticks,
          start: secondsAt(note.start_tick, loaded.score),
          end: secondsAt(note.start_tick + note.duration_ticks, loaded.score),
        };
      }),
    )
    .sort((a, b) => a.startTick - b.startTick || a.endTick - b.endTick);
  let endTick = 0;
  for (const note of notes) endTick = Math.max(endTick, note.endTick);
  const duration = secondsAt(endTick, loaded.score);
  const playbackError =
    notes.some((n) => !Number.isFinite(n.end) || n.end <= n.start) ||
    !Number.isFinite(duration) ||
    duration > Number.MAX_SAFE_INTEGER
      ? "此 Score 有效，但其时间无法在本地试听时钟中可靠表示。仍可查看 tick 网格。"
      : null;
  return {
    ...loaded,
    notes,
    endTick,
    duration,
    playbackError,
    index: new NoteIndex(notes),
  };
}
export function activeKeys(score: PreparedScore, seconds: number): number[] {
  const tick = tickAt(seconds, score.score);
  return [...new Set(score.index.between(tick, tick).map((n) => n.key))].sort(
    (a, b) => a - b,
  );
}
export function lowerBound(notes: PlayerNote[], time: number): number {
  let lo = 0,
    hi = notes.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (notes[mid].start < time) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}
export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds)) return "—";
  const whole = Math.max(0, Math.floor(seconds));
  return `${String(Math.floor(whole / 60)).padStart(2, "0")}:${String(whole % 60).padStart(2, "0")}`;
}
