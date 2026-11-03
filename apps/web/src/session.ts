import type { Preview, Receipt, Scope, Transport } from './types.ts';
export type SessionState = { generation: number; libraryPath: string; scope: Scope; projectRoot: string | null; selection: string[]; phase: 'idle' | 'planning' | 'ready' | 'confirming' | 'expired' | 'result'; preview: Preview | null; receipt: Receipt | null; error: string | null };
export class SessionController {
  state: SessionState = { generation: 0, libraryPath: '', scope: 'user', projectRoot: null, selection: [], phase: 'idle', preview: null, receipt: null, error: null };
  call: Transport;
  now: () => number;
  constructor(call: Transport, now = () => Date.now() / 1000) { this.call = call; this.now = now; }
  select(_key: string, _checked: boolean) { throw new Error('not_implemented'); }
  async context(_update: Partial<Pick<SessionState, 'libraryPath' | 'scope' | 'projectRoot'>>) { throw new Error('not_implemented'); }
  async pickProject() { throw new Error('not_implemented'); }
  async read<T>(_command: string, _args?: Record<string, unknown>): Promise<T | null> { throw new Error('not_implemented'); }
  async prepare(_kind: string, _args: Record<string, unknown> = {}) { throw new Error('not_implemented'); }
  async confirm() { throw new Error('not_implemented'); }
  expire() { throw new Error('not_implemented'); }
}
