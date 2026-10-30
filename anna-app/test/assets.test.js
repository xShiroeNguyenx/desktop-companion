import { test } from 'node:test';
import assert from 'node:assert/strict';
import { seriesOrder } from '../scripts/sync-assets.mjs';

test('models sort by line, then purple → pink → snow, like the desktop Collection', () => {
  const ids = ['snow-dancer', 'pink-angel', 'snow-pastel', 'gothic-lolita', 'pink-kawaii', 'gothic-aristocrat', 'gothic-kawaii', 'custom-x'];
  assert.deepEqual(ids.sort(seriesOrder), [
    'gothic-kawaii', 'pink-kawaii', 'snow-pastel', 'gothic-aristocrat', 'pink-angel', 'gothic-lolita', 'snow-dancer', 'custom-x',
  ]);
});
