import type { Pair } from './types.ts';
export function cellKey(packageId: string, agent: string): string { return JSON.stringify([packageId, agent]); }
export function selectionFromCells(cells: Iterable<string>): Pair[] {
  const pairs = new Map<string, Pair>();
  for (const cell of cells) {
    const value: unknown = JSON.parse(cell);
    if (!Array.isArray(value) || value.length !== 2 || !value.every(x => typeof x === 'string')) throw new Error('invalid_selection');
    const [packageId, agent] = value as string[];
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(packageId) || packageId.length > 128 || agent.length > 64 || !agent.split('.').every(s => /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(s))) throw new Error('invalid_selection');
    pairs.set(cellKey(packageId, agent), { packageId, agent });
    if (pairs.size > 4096) throw new Error('too_many_pairs');
  }
  return [...pairs.values()];
}
