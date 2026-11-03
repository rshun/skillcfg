import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { parsePreview } from '../src/contracts.ts';
test('Rust distribution fixture crosses the frontend contract without losing evidence', async () => {
  const data = JSON.parse(await readFile(new URL('../../../tests/fixtures/contracts/distribution.json', import.meta.url), 'utf8'));
  assert.deepEqual(parsePreview(data), data);
});
test('incomplete preview cannot reach confirmation', () => {
  for (const value of [{}, { previewId: 'forged', groups: [], blocked: false }]) assert.throws(() => parsePreview(value));
});
