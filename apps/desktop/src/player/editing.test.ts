import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { Score } from "./model.ts";
import { prepareScore } from "./model.ts";
import {
  cellTick,
  placementError,
  putNote,
  removeNote,
  resizeNote,
  snapDuration,
  snapTick,
  notesInCell,
  toggleNoteAtCell,
} from "./editing.ts";
import type { ScoreNote } from "./editing.ts";
const fixture = (): Score =>
  JSON.parse(
    readFileSync(
      new URL("../../../../docs/examples/sky-15-example.json", import.meta.url),
      "utf8",
    ),
  );
const note = (change: Partial<ScoreNote> = {}): ScoreNote => ({
  id: "new",
  pitch: "A4",
  start_tick: 240,
  duration_ticks: 240,
  ...change,
});

test("keyboard toggles an empty cell on and off without mutating its undo snapshot", () => {
  const score = fixture();
  const added = toggleNoteAtCell(score, 0, "A4", 240, 240, "added");
  assert.equal(added.selected, "added");
  assert.equal(
    added.score.tracks[0].notes.find((n) => n.id === "added")?.duration_ticks,
    240,
  );
  const removed = toggleNoteAtCell(added.score, 0, "A4", 240, 240, "unused");
  assert.equal(removed.selected, null);
  assert.deepEqual(removed.score, score);
  assert.ok(added.score.tracks[0].notes.some((n) => n.id === "added"));
});

test("keyboard toggle removes the full sustained or off-grid note intersecting the cell", () => {
  const score = putNote(
    fixture(),
    note({ start_tick: 250, duration_ticks: 1000 }),
    0,
  );
  assert.ok(notesInCell(score, 0, 240, 240).some((n) => n.id === "new"));
  for (const tick of [240, 480, 1200]) {
    const removed = toggleNoteAtCell(score, 0, "A4", tick, 240, "unused");
    assert.ok(!removed.score.tracks[0].notes.some((n) => n.id === "new"));
  }
  assert.ok(!notesInCell(score, 0, 0, 240).some((n) => n.id === "new"));
  assert.ok(!notesInCell(score, 0, 1250, 240).some((n) => n.id === "new"));
});

test("keyboard deletion is scoped to the edit track and leaves other tracks intact", () => {
  const score = fixture();
  const removed = toggleNoteAtCell(score, 0, "C4", 0, 240, "unused");
  assert.ok(!removed.score.tracks[0].notes.some((n) => n.id === "note-001"));
  assert.deepEqual(removed.score.tracks[1], score.tracks[1]);
  assert.throws(
    () => toggleNoteAtCell(removed.score, 0, "C4", 0, 240, "new"),
    /相同音高/,
  );
  assert.ok(!notesInCell(score, 0, 480, 240).some((n) => n.id === "note-001"));
});

test("grid chooses cell starts, positive duration multiples and integer drag positions", () => {
  assert.equal(cellTick(479.9, 240), 240);
  assert.equal(snapTick(479.9, 240), 480);
  assert.equal(snapTick(-50, 240), 0);
  for (const length of [-500, 0, 30, 250, 603]) {
    const quantized = snapDuration(length, 240);
    assert.ok(quantized >= 240);
    assert.equal(quantized % 240, 0);
  }
});
test("adding, moving and pasting reject same-pitch overlap across all tracks, but allow adjacent notes and chords", () => {
  const score = fixture();
  assert.ok(placementError(score, note({ pitch: "C4", start_tick: 480 })));
  assert.equal(
    placementError(score, note({ pitch: "C4", start_tick: 960 })),
    null,
  );
  assert.equal(placementError(score, note()), null);
  const next = putNote(score, note(), 0);
  assert.throws(() => putNote(next, note({ id: "copy" }), 1), /相同音高/);
  assert.equal(placementError(next, note({ start_tick: 480 }), "new"), null);
  const moved = putNote(next, note({ start_tick: 480 }), 0, "new");
  assert.equal(
    moved.tracks[0].notes.find((n) => n.id === "new")!.start_tick,
    480,
  );
  assert.equal(
    next.tracks[0].notes.find((n) => n.id === "new")!.start_tick,
    240,
  );
});
test("edits preserve source metadata, timing and legacy overlaps without mutating the original", () => {
  const score = fixture(),
    before = JSON.stringify(score);
  const next = putNote(score, note(), 1);
  assert.equal(JSON.stringify(score), before);
  assert.equal(next.timeline, score.timeline);
  assert.equal(next.instrument, score.instrument);
  assert.equal(next.text, score.text);
  assert.deepEqual(removeNote(next, "new"), score);
  assert.throws(() => putNote(score, note({ id: "note-001" }), 0), /标识重复/);
  assert.throws(() => putNote(score, note(), 8), /轨道/);
});
test("resize keeps positive quantized lengths, anchors opposite edge and prevents negative start", () => {
  const source = note({ start_tick: 960, duration_ticks: 480 });
  assert.deepEqual(resizeNote(source, "start", 100, 240), {
    ...source,
    start_tick: 0,
    duration_ticks: 1440,
  });
  assert.deepEqual(resizeNote(source, "start", 2000, 240), {
    ...source,
    start_tick: 1200,
    duration_ticks: 240,
  });
  assert.deepEqual(resizeNote(source, "end", 1760, 240), {
    ...source,
    duration_ticks: 720,
  });
  assert.equal(resizeNote(source, "end", 0, 240).duration_ticks, 240);
  assert.equal(
    resizeNote(note({ start_tick: 0, duration_ticks: 1 }), "start", 0, 240)
      .duration_ticks,
    1,
  );
});
test("invalid pitch and unsafe tick arithmetic are rejected before changing the document", () => {
  const score = fixture();
  for (const change of [
    { pitch: "A#4" },
    { start_tick: -1 },
    { start_tick: 0.5 },
    { duration_ticks: 0 },
    { duration_ticks: Infinity },
    { start_tick: Number.MAX_SAFE_INTEGER },
  ]) {
    assert.ok(placementError(score, note(change)));
    assert.throws(() => putNote(score, note(change), 0));
  }
  assert.equal(
    placementError(score, note({ start_tick: Number.MAX_SAFE_INTEGER - 240 })),
    null,
  );
});
test("prepared playback index reflects edits and removal immediately", () => {
  const score = putNote(
    fixture(),
    note({ start_tick: 3840, duration_ticks: 960 }),
    0,
  );
  const loaded = {
    score,
    source: "fixture",
    midi_keys: [60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84],
  };
  const song = prepareScore(loaded);
  assert.equal(song.duration, 2.5);
  assert.deepEqual(
    song.index.between(3900, 3900).map((n) => n.id),
    ["new"],
  );
  assert.equal(
    prepareScore({ ...loaded, score: removeNote(score, "new") }).duration,
    1,
  );
});
