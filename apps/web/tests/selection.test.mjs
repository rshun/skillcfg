import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cellKey, selectionFromCells } from '../src/selection.ts';
test('selection preserves exact pairs instead of a Cartesian product', () => {
  assert.deepEqual(selectionFromCells([cellKey('one', 'codex'), cellKey('two', 'grok'), cellKey('one', 'codex')]), [{ packageId: 'one', agent: 'codex' }, { packageId: 'two', agent: 'grok' }]);
});
test('fifth agent is accepted without an enumeration change', () => {
  assert.deepEqual(selectionFromCells([cellKey('one', 'custom.fifth')]), [{ packageId: 'one', agent: 'custom.fifth' }]);
});
test('forged or malformed selection cells are rejected', () => {
  for (const cell of ['../path', '["one","../evil"]', '["one",null]', '["../one","codex"]']) assert.throws(() => selectionFromCells([cell]));
});
