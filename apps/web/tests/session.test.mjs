import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SessionController } from '../src/session.ts';
import { cellKey } from '../src/selection.ts';
const preview = (id = 'p', expiresAt = 600) => ({ previewId: id, expiresAt, groups: [], warnings: [], blocked: false, backupLocation: '/backups' });
function fixture(impl = async (cmd) => cmd.startsWith('preview_') ? preview() : null, now = () => 10) {
  const calls = []; const session = new SessionController(async (c, a) => { calls.push([c, a]); return impl(c, a); }, now); return { session, calls };
}
test('switching context clears all pairs and the ready preview', async () => {
  const { session } = fixture(); session.select(cellKey('one', 'codex'), true); await session.prepare('distribution'); await session.context({ scope: 'project', projectRoot: '/project' });
  assert.deepEqual(session.state.selection, []); assert.equal(session.state.preview, null); assert.equal(session.state.scope, 'project');
});
test('cancelling project picker keeps context and current selection', async () => {
  const { session } = fixture(); session.select(cellKey('one', 'codex'), true); await session.pickProject(); assert.equal(session.state.scope, 'user'); assert.equal(session.state.selection.length, 1);
});
test('slow old agent response is ignored after context change', async () => {
  let release; const { session } = fixture(async (cmd) => cmd === 'list_agent_skills' ? new Promise(r => { release = r; }) : null);
  const old = session.read('list_agent_skills'); await session.context({ scope: 'project', projectRoot: '/other' }); release({ old: true }); assert.equal(await old, null);
});
test('old planning response cannot become ready in a new context', async () => {
  let release; const { session } = fixture(async (cmd) => cmd.startsWith('preview_') ? new Promise(r => { release = r; }) : null);
  const old = session.prepare('distribution'); await session.context({ libraryPath: '/other' }); release(preview()); await old; assert.equal(session.state.preview, null); assert.notEqual(session.state.phase, 'ready');
});
test('double confirmation is never replayed and context is locked during commit', async () => {
  let release; const { session, calls } = fixture(async c => c.startsWith('preview_') ? preview() : c.startsWith('commit_') ? new Promise(r => { release = r; }) : null);
  await session.prepare('distribution'); const first = session.confirm(); await session.confirm(); await session.context({ scope: 'project', projectRoot: '/other' });
  assert.equal(calls.filter(([c]) => c === 'commit_distribution').length, 1); assert.equal(session.state.scope, 'user');
  release({ operationId: 'op', backupLocation: '/backups', outcomes: [] }); await first;
});
test('expired preview has no usable confirmation', async () => {
  const { session, calls } = fixture(async () => preview('expired', 1)); await session.prepare('distribution'); session.expire(); await session.confirm();
  assert.equal(session.state.phase, 'expired'); assert.equal(calls.some(([c]) => c.startsWith('commit_')), false);
});
test('partial failure preserves successful group receipts', async () => {
  const receipt = { operationId: 'op', backupLocation: '/backups', outcomes: [{ path: '/one', outcome: 'written', reason: null }, { path: '/two', outcome: 'failed_restored', reason: 'permission_denied' }] };
  const { session } = fixture(async c => c.startsWith('preview_') ? preview() : receipt); await session.prepare('distribution'); await session.confirm(); assert.deepEqual(session.state.receipt, receipt); assert.equal(session.state.phase, 'result');
});
test('restore uses an independent one-time preview and only its ID crosses commit', async () => {
  const { session, calls } = fixture(); await session.prepare('distribution'); await session.prepare('restore', { operationId: 'old' }); await session.confirm();
  assert.deepEqual(calls.find(([c]) => c === 'commit_restore'), ['commit_restore', { previewId: 'p' }]); assert.equal(calls.some(([c]) => c === 'commit_distribution'), false);
});
test('registry invalidation clears pairs while preserving a completed receipt', async () => {
  const receipt={operationId:'op',backupLocation:'/backups',outcomes:[{path:'/one',outcome:'written',reason:null}]};
  const {session}=fixture(async c=>c.startsWith('preview_')?preview():receipt);
  session.select(cellKey('one','codex'),true);await session.prepare('distribution');await session.confirm();await session.clear();
  assert.deepEqual(session.state.receipt,receipt);assert.deepEqual(session.state.selection,[]);
});
test('rejected native context switch cannot leave the UI claiming a different scope', async () => {
  const {session}=fixture(async c=>{if(c==='set_context')throw {code:'invalid_project'};return null;});
  assert.equal(await session.context({scope:'project',projectRoot:'/missing'}),false);assert.equal(session.state.scope,'user');assert.equal(session.state.projectRoot,null);
});
