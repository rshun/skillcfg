import type { Preview } from './types.ts';
export function parsePreview(value: unknown): Preview {
  const p = value as Preview;
  if (!p || typeof p.previewId !== 'string' || !p.previewId || !Number.isFinite(p.expiresAt) || typeof p.blocked !== 'boolean' || typeof p.backupLocation !== 'string' || !Array.isArray(p.groups) || !Array.isArray(p.warnings) || !p.warnings.every(w => typeof w === 'string')) throw new Error('invalid_preview_response');
  for (const g of p.groups) {
    if (typeof g.targetKey !== 'string' || typeof g.path !== 'string' || !['new', 'update', 'unchanged', 'skipped', 'blocked'].includes(g.disposition) || !Array.isArray(g.selectedAgents) || !g.selectedAgents.every(a => typeof a === 'string') || !Array.isArray(g.possibleReaders) || !Array.isArray(g.files)) throw new Error('invalid_preview_response');
    for (const r of g.possibleReaders) if (typeof r.agent !== 'string' || typeof r.variant !== 'string' || typeof r.rootId !== 'string' || !['declared', 'configEnabled', 'configDisabled', 'unknown'].includes(r.evidence)) throw new Error('invalid_reader_response');
    for (const f of g.files) if (typeof f.path !== 'string' || typeof f.binary !== 'boolean' || typeof f.truncated !== 'boolean' || typeof f.beforeBytes !== 'number' || typeof f.afterBytes !== 'number' || ![f.before, f.after].every(t => t === null || typeof t === 'string')) throw new Error('invalid_diff_response');
  }
  return p;
}
