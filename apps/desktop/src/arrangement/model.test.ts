import {test} from 'node:test';
import assert from 'node:assert/strict';
import {defaultArrangementOptions as defaults,optionsError} from './model.ts';
test('arrangement controls accept defaults and reject fractional/empty/out-of-range constraints',()=>{
  assert.equal(optionsError(defaults),null);
  for(const maximum_polyphony of [0,129,1.5,NaN]) assert.ok(optionsError({...defaults,maximum_polyphony}));
  for(const priority of [-1,6,1.1,NaN]) {
    for(const key of ['melody_priority','bass_priority','accompaniment_priority']) assert.ok(optionsError({...defaults,[key]:priority}));
  }
  for(const maximum_polyphony of [null,1,128]) assert.equal(optionsError({...defaults,maximum_polyphony}),null);
});
