import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  activeKeys,
  meterTicks,
  prepareScore,
  secondsAt,
  tickAt,
} from "./model.ts";
import type { LoadedScore } from "./model.ts";

function example(): LoadedScore {
  return {
    score: JSON.parse(
      readFileSync(
        new URL(
          "../../../../docs/examples/sky-15-example.json",
          import.meta.url,
        ),
        "utf8",
      ),
    ),
    source: "test fixture",
    midi_keys: [60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84],
  };
}
test("real fixture: quarter-note tempo, chords, overlap, exact ends", () => {
  const prepared = prepareScore(example());
  assert.equal(prepared.duration, 1);
  assert.deepEqual(activeKeys(prepared, 0), [0, 2, 4]);
  assert.deepEqual(activeKeys(prepared, 0.25), [0, 2, 4]);
  assert.deepEqual(activeKeys(prepared, 0.5), [4]);
  assert.deepEqual(activeKeys(prepared, 1), []);
  assert.equal(
    tickAt(secondsAt(123456, prepared.score), prepared.score),
    123456,
  );
});
test("overlapping/repeated same-key notes do not extinguish each other", () => {
  const loaded = example();
  loaded.score.tracks = [
    {
      id: "t",
      name: "t",
      role: "melody",
      notes: [
        { id: "a", pitch: "C4", start_tick: 0, duration_ticks: 1920 },
        { id: "b", pitch: "C4", start_tick: 480, duration_ticks: 480 },
        { id: "c", pitch: "C4", start_tick: 1920, duration_ticks: 480 },
      ],
    },
  ];
  const score = prepareScore(loaded);
  for (const time of [0, 0.25, 0.5, 0.999, 1])
    assert.deepEqual(activeKeys(score, time), [0]);
  assert.deepEqual(activeKeys(score, 1.25), []);
});
test("nominal meter supports 6/8 and 7/16 without changing tempo", () => {
  const { score } = example();
  score.timeline.time_signature = { numerator: 6, denominator: 8 };
  assert.deepEqual(meterTicks(score), { beat: 480, bar: 2880 });
  assert.equal(secondsAt(960, score), 0.5);
  score.timeline.time_signature = { numerator: 7, denominator: 16 };
  assert.deepEqual(meterTicks(score), { beat: 240, bar: 1680 });
});
test("custom instrument preserves exact spelling and ordered keys", () => {
  const loaded = example();
  loaded.score.instrument = {
    id: "custom",
    name: "custom",
    playback_mode: "sustain",
    keys: ["B#3", "C4"],
  };
  loaded.midi_keys = [60, 60];
  loaded.score.tracks = [
    {
      id: "t",
      name: "t",
      role: "melody",
      notes: [{ id: "n", pitch: "C4", start_tick: 0, duration_ticks: 960 }],
    },
  ];
  assert.deepEqual(activeKeys(prepareScore(loaded), 0), [1]);
});
test("empty score and valid but unrepresentable playback times are explicit", () => {
  const loaded = example();
  loaded.score.timeline.tempo = Number.MIN_VALUE;
  assert.match(prepareScore(loaded).playbackError!, /时间/);
  loaded.score.tracks = [];
  assert.equal(prepareScore(loaded).duration, 0);
  assert.equal(prepareScore(loaded).playbackError, null);
});
test("interval queries include long sustains and all unsorted source events", () => {
  const loaded = example();
  loaded.score.tracks[0].notes.reverse();
  const prepared = prepareScore(loaded);
  assert.equal(prepared.index.between(480, 480).length, 4);
  assert.equal(prepared.index.between(1920, 2000).length, 0);
});
