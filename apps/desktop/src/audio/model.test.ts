import test from "node:test";
import assert from "node:assert/strict";
import { audioOptionsError, defaultAudioOptions, progressPercent } from "./model.ts";

test("audio settings reject blanks, infinities and unbounded note extraction", () => {
  assert.equal(audioOptionsError(defaultAudioOptions), null);
  for (const value of [NaN, Infinity, -1, 0, 1]) {
    assert.ok(audioOptionsError({...defaultAudioOptions, frame_threshold: value}));
  }
  for (const value of [NaN, Infinity, 0, 1001]) {
    assert.ok(audioOptionsError({...defaultAudioOptions, minimum_note_ms: value}));
  }
});
test("progress uses measured work and leaves unknown totals indeterminate", () => {
  assert.equal(progressPercent({stage:"model",completed:0,total:null}), undefined);
  assert.equal(progressPercent({stage:"transcribe",completed:3,total:4}), 75);
  assert.equal(progressPercent({stage:"decode",completed:5,total:4}), 100);
  assert.equal(progressPercent({stage:"decode",completed:5,total:0}), undefined);
});
