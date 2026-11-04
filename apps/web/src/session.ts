import type { Preview, Receipt, Scope, Transport } from './types.ts';
import { parsePreview } from './contracts.ts';
import { selectionFromCells } from './selection.ts';
export type SessionState = { generation: number; libraryPath: string; scope: Scope; projectRoot: string | null; selection: string[]; kind: string | null; phase: 'idle' | 'planning' | 'ready' | 'confirming' | 'expired' | 'result'; preview: Preview | null; receipt: Receipt | null; error: string | null };
export function errorCode(error: unknown): string {
  const code = (error as { code?: unknown })?.code;
  return typeof code === 'string' && /^[a-z_]{1,80}$/.test(code) ? code : 'operation_failed';
}
export class SessionController {
  state: SessionState = { generation: 0, libraryPath: '', scope: 'user', projectRoot: null, selection: [], kind: null, phase: 'idle', preview: null, receipt: null, error: null };
  call: Transport;
  now: () => number;
  private listeners = new Set<() => void>();
  private epoch = 0;
  constructor(call: Transport, now = () => Date.now() / 1000) { this.call = call; this.now = now; }
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  snapshot = () => this.state;
  private update(change: Partial<SessionState>) { this.state = { ...this.state, ...change }; for (const fn of this.listeners) fn(); }
  select(key: string, checked: boolean) {
    if (this.state.phase === 'confirming') return;
    selectionFromCells([key]); const cells = new Set(this.state.selection); if (checked) cells.add(key); else cells.delete(key);
    const oldId=this.state.preview?.previewId;
    ++this.epoch; this.update({ selection: [...cells], preview: null, phase: 'idle', error: null });
    if(oldId)void this.call('cancel_preview',{previewId:oldId}).catch(()=>{});
  }
  async clear() { if (this.state.phase === 'confirming') return; if (this.state.phase !== 'result') await this.cancel(); this.update({ selection: [] }); }
  async cancel() {
    if (this.state.phase === 'confirming') return;
    const id = this.state.preview?.previewId; ++this.epoch; this.update({ preview: null, phase: 'idle', receipt: null, error: null, kind: null });
    if (id) { try { await this.call('cancel_preview', { previewId: id }); } catch { /* Local preview is already unusable. */ } }
  }
  async context(update: Partial<Pick<SessionState, 'libraryPath' | 'scope' | 'projectRoot'>>) {
    if (this.state.phase === 'confirming') return false;
    const previous = { libraryPath: this.state.libraryPath, scope: this.state.scope, projectRoot: this.state.projectRoot };
    const next = { ...this.state, ...update };
    if (next.scope === 'project' && !next.projectRoot) return false;
    ++this.epoch;
    this.update({ ...update, generation: this.state.generation + 1, selection: [], preview: null, phase: 'idle', receipt: null, error: null, kind: null });
    const generation = this.state.generation;
    try { await this.call('set_context', { scope: next.scope, projectRoot: next.projectRoot }); return generation === this.state.generation; }
    catch (error) { if (generation === this.state.generation) this.update({ ...previous, error: errorCode(error) }); return false; }
  }
  async pickProject() {
    if (this.state.phase === 'confirming') return false;
    const generation = this.state.generation;
    const picked = await this.call<string | null>('choose_directory', { initial: this.state.projectRoot });
    if (!picked || generation !== this.state.generation || this.snapshot().phase === 'confirming') return false;
    return this.context({ scope: 'project', projectRoot: picked });
  }
  async read<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
    const generation = this.state.generation; const result = await this.call<T>(command, args);
    return generation === this.state.generation ? result : null;
  }
  async prepare(kind: string, args: Record<string, unknown> = {}) {
    if (this.state.phase === 'confirming') return;
    if (!['distribution', 'import', 'restore'].includes(kind)) throw new Error('invalid_preview_kind');
    const epoch = ++this.epoch, generation = this.state.generation;
    this.update({ phase: 'planning', preview: null, receipt: null, error: null, kind });
    try {
      const preview = parsePreview(await this.call(`preview_${kind}`, args));
      if (epoch !== this.epoch || generation !== this.state.generation) { await this.call('cancel_preview',{previewId:preview.previewId}).catch(()=>{});return; }
      this.update({ preview, phase: preview.expiresAt <= this.now() ? 'expired' : 'ready' });
    } catch (error) { if (epoch === this.epoch && generation === this.state.generation) this.update({ phase: 'idle', error: errorCode(error) }); }
  }
  expire() { if (this.state.phase === 'ready' && this.state.preview && this.state.preview.expiresAt <= this.now()) this.update({ phase: 'expired' }); }
  async confirm() {
    this.expire(); if (this.state.phase !== 'ready' || !this.state.preview || this.state.preview.blocked) return;
    const { previewId } = this.state.preview, kind = this.state.kind;
    this.update({ phase: 'confirming', error: null });
    try { const receipt = await this.call<Receipt>(`commit_${kind}`, { previewId }); this.update({ phase: 'result', receipt, preview: null }); }
    catch (error) { this.update({ phase: 'result', preview: null, receipt: null, error: errorCode(error) }); }
  }
}
