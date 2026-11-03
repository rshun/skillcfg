import type { Pair } from './types.ts';
export function cellKey(packageId: string, agent: string): string { return JSON.stringify([packageId, agent]); }
export function selectionFromCells(_cells: Iterable<string>): Pair[] { throw new Error('not_implemented'); }
