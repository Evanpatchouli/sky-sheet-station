import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PlayerEngine } from "./engine.ts";
import { prepareScore } from "./model.ts";

class FakeAudioContext {
  static latest: FakeAudioContext;
  currentTime = 0;
  state = "running";
  destination = {};
  onstatechange: (() => void) | null = null;
  oscillators: {
    starts: number[];
    stops: number[];
    frequency: { value: number };
  }[] = [];
  constructor() {
    FakeAudioContext.latest = this;
  }
  async resume() {
    this.state = "running";
  }
  async close() {
    this.state = "closed";
  }
  createGain() {
    return {
      connect() {},
      disconnect() {},
      gain: {
        value: 0,
        setValueAtTime() {},
        linearRampToValueAtTime() {},
        exponentialRampToValueAtTime() {},
        setTargetAtTime() {},
      },
    };
  }
  createDynamicsCompressor() {
    return { connect() {} };
  }
  createOscillator() {
    const node = {
      type: "",
      frequency: { value: 0 },
      onended: null,
      starts: [] as number[],
      stops: [] as number[],
      connect() {},
      disconnect() {},
      start(time: number) {
        this.starts.push(time);
      },
      stop(time = 0) {
        this.stops.push(time);
      },
    };
    this.oscillators.push(node);
    return node;
  }
}
function loadedEngine() {
  const score = JSON.parse(
    readFileSync(
      new URL("../../../../docs/examples/sky-15-example.json", import.meta.url),
      "utf8",
    ),
  );
  const engine = new PlayerEngine();
  engine.load(
    prepareScore({
      score,
      midi_keys: [60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84],
      source: "fixture",
    }),
  );
  return engine;
}
test("audio clock anchors pause/resume/seek/speed and stops all scheduled voices", async (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  t.mock.method(globalThis, "AudioContext" as never, FakeAudioContext as never);
  const engine = loadedEngine();
  t.after(() => engine.dispose());
  engine.pause();
  assert.equal(engine.snapshot().status, "stopped");
  assert.deepEqual(engine.snapshot().keys, []);
  await engine.play();
  const clock = FakeAudioContext.latest;
  assert.equal(clock.oscillators.length, 4); // melody plus 3-note chord at zero
  assert.deepEqual(engine.snapshot().keys, [0, 2, 4]);
  clock.currentTime = 0.2;
  t.mock.timers.tick(25);
  assert.equal(engine.snapshot().position, 0.2);
  engine.pause();
  clock.currentTime = 10;
  t.mock.timers.tick(250);
  assert.equal(engine.snapshot().position, 0.2);
  engine.seek(0.75);
  assert.deepEqual(engine.snapshot().keys, [4]);
  await engine.play();
  clock.currentTime = 10.1;
  t.mock.timers.tick(25);
  assert.ok(Math.abs(engine.snapshot().position - 0.85) < 1e-10);
  engine.setSpeed(0.5);
  clock.currentTime = 10.2;
  t.mock.timers.tick(25);
  assert.ok(Math.abs(engine.snapshot().position - 0.9) < 1e-10);
  engine.stop();
  assert.equal(engine.snapshot().position, 0);
  assert.deepEqual(engine.snapshot().keys, []);
  assert.ok(clock.oscillators.every((n) => n.stops.length >= 2));
});
test("end, replay, loop overshoot and pending-play cancellation", async (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  t.mock.method(globalThis, "AudioContext" as never, FakeAudioContext as never);
  const engine = loadedEngine();
  t.after(() => engine.dispose());
  const pending = engine.play();
  engine.stop();
  await pending;
  assert.equal(engine.snapshot().status, "stopped");
  await engine.play();
  const clock = FakeAudioContext.latest;
  clock.currentTime = 1.1;
  t.mock.timers.tick(25);
  assert.equal(engine.snapshot().status, "stopped");
  assert.equal(engine.snapshot().position, 1);
  engine.setLoop(true);
  await engine.play();
  clock.currentTime = 2.25;
  t.mock.timers.tick(25);
  assert.equal(engine.snapshot().status, "playing");
  assert.ok(Math.abs(engine.snapshot().position - 0.15) < 1e-10);
  clock.state = "suspended";
  clock.onstatechange?.();
  assert.equal(engine.snapshot().status, "paused");
});

// Web Audio does not exist in Node. The fake only supplies its clock/node API;
// the production engine and the repository score are exercised unchanged.
Object.defineProperty(globalThis, "AudioContext", {
  value: FakeAudioContext,
  writable: true,
  configurable: true,
});
