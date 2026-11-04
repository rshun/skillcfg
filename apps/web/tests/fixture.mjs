export function installFixture() {
  window.calls = []; window.confirm = () => window.acceptDiscard !== false;
  const settings = { version: 1, libraryPath: '/fixture/library', backupPath: '/fixture/backups', defaultScope: 'user', lastProjectDir: null, agentVariants: {} };
  const names = ['Codex', 'Claude Code', 'Antigravity', 'Grok Build']; const ids = ['codex', 'claude-code', 'antigravity', 'grok'];
  const record = { id: 'package-one', currentRevision: 'revision-one', name: 'code-review', displayName: 'code-review', description: '审阅变更，核对边界与回归证据。', sourceKind: 'agent', sourceAgent: 'claude-code', sourcePath: '/fixture/source/code-review', importedAt: 1793680000 };
  let records = [record, { ...record, id: 'package-two', name: 'incident-response', displayName: 'incident-response', description: '保留现场，整理线索与处置步骤。' }, { ...record, id: 'package-three', name: 'writing-plans', displayName: 'writing-plans', description: '将目标整理为可验证的实施计划。' }];
  let counter = 0;const disabled=new Set();
  const preview = (request = {}) => ({ previewId: `preview-${++counter}`, expiresAt: window.expired ? 1 : Date.now() / 1000 + 600, backupLocation: '/fixture/backups', blocked: false, warnings: [], groups: [{ targetKey: 'physical-one', path: '/fixture/.agents/skills/code-review', disposition: 'update', selectedAgents: (request.selection || [{ agent: 'codex' }]).map(x => x.agent), possibleReaders: [{ agent: window.customReader?'custom.fifth':'antigravity', variant: 'default', rootId: 'shared', evidence: 'unknown' }], files: [{ path: 'SKILL.md', binary: false, before: 'Earlier text', after: 'Reviewed text', beforeBytes: 12, afterBytes: 13, truncated: false }, { path: 'old-extra.txt', binary: false, before: 'Retained in archive', after: null, beforeBytes: 18, afterBytes: 0, truncated: false }], reason: null }] });
  const receipt = () => ({ operationId: 'operation-one', backupLocation: '/fixture/backups/operation-one', outcomes: [{ path: '/fixture/.agents/skills/code-review', outcome: 'written', reason: null }, ...(window.partial ? [{ path: '/fixture/second', outcome: 'failed_restored', reason: 'permission_denied' }] : [])] });
  window.__TAURI_INTERNALS__ = { async invoke(command, args) {
    if (command !== 'execute') throw { code: 'unknown_command' };
    const { command: name, request = {} } = args; window.calls.push([name, structuredClone(request)]);
    switch (name) {
      case 'app_info': return { version: 'fixture', platform: 'linux', mode: 'fixture' };
      case 'load_settings': if (window.badSettings) throw { code: 'invalid_settings' }; return { settings, revision: 'settings-one', warnings: [] };
      case 'load_library': if (window.badLibrary) throw { code: 'invalid_catalog' }; return { firstUse: !!window.emptyLibrary, records: window.emptyLibrary ? [] : records };
      case 'list_agent_profiles': return [...ids, ...(window.fifth ? ['custom.fifth'] : [])].map((id, i) => ({ id, displayName: names[i] || 'Fifth Agent', revision: 'profile-one', source: i < 4 ? 'builtin' : 'user', state: disabled.has(id)?'disabled':'builtin', variants: ['default'], currentVariant: 'default', roots: [{ agent: id, rootId: 'native', variant: 'default', path: `/fixture/${id}/skills`, actualPath: `/fixture/${id}/skills`, rootWasLink: false, writable: true, priority: 100 }], issues: [] })).concat(window.invalidProfile?[{id:null,displayName:'损坏档案',revision:null,source:'user',state:'invalid',variants:[],currentVariant:null,roots:[],issues:['invalid_profile']}]:[]);
      case 'library_statuses': return records.flatMap((r, i) => ids.map((agentId, n) => ({ packageId: r.id, agentId, state: i === 0 && n === 0 ? 'equal' : i === 1 && n === 0 ? 'different' : 'missing', path: `/fixture/${agentId}/skills/${r.name}` })));
      case 'detect_agents': return ids.map(agentId => ({ agentId, installed: true, issues: [] }));
      case 'read_package': return { packageId: request.packageId, revisionId: 'revision-one', document: '---\nname: code-review\ndescription: Review changes.\n---\nRead every changed line.\n', editToken: 'edit-one', files: ['SKILL.md', 'references/checklist.md'], issues: [] };
      case 'choose_directory': return window.cancelPicker ? null : '/fixture/中文项目';
      case 'set_context': return null;
      case 'list_agent_skills': if (window.slowInventory) return new Promise(resolve => { window.releaseInventory = () => resolve({ agentId: request.agentId, installed: true, roots: [], items: [{ name: 'old-response', description: 'Old', sources: [], files: [], issues: [] }], errors: [] }); }); return { agentId: request.agentId, installed: true, roots: [], items: [{ name: 'code-review', description: 'Review changes.', sources: [{ path: '/fixture/source-link', actualPath: '/fixture/source/code-review', rootWasLink: true }], files: ['SKILL.md'], issues: [] }], errors: [] };
      case 'scan_import': return { scanId: 'scan-one', items: [{ itemId: 'item-one', name: 'imported-skill', sourcePath: request.source, actualPath: request.source, issues: [] }] };
      case 'preview_import': return preview(request);
      case 'commit_import': records = [...records, { ...record, id: 'package-imported', name: 'imported-skill', displayName: 'imported-skill' }]; window.emptyLibrary = false; return receipt();
      case 'save_skill_document': if (window.failSave) throw { code: 'write_failed' }; return receipt();
      case 'save_settings': Object.assign(settings, request.settings); return { settings, revision: 'settings-two', warnings: [] };
      case 'read_agent_profile': return { profileJson: JSON.stringify({ schemaVersion: 1, id: 'custom.fifth', displayName: 'Fifth Agent', packageFormat: 'skill-md-v1', variants: ['default'], defaultVariant: 'default', roots: [], probes: [], binaryNames: [] }, null, 2), revision: null };
      case 'preview_agent_profile': return { previewId: 'profile-preview', profileId: 'custom.fifth', resolvedRoots: ['/fixture/.fifth/skills'], probePaths: ['/fixture/.fifth/config.json'], variants: ['default'], blocked: false, warnings: [] };
      case 'commit_agent_profile': window.fifth = true; return receipt();
      case 'set_agent_profile_enabled': disabled.add(request.agentId);return receipt();
      case 'preview_distribution': return preview(request);
      case 'commit_distribution': if (window.slowCommit) await new Promise(resolve => { window.releaseCommit = resolve; }); return receipt();
      case 'cancel_preview': return null;
      case 'list_history': return [{ operationId: 'old-operation', kind: 'distribution', createdAt: 1793680000, backupLocation: '/fixture/backups/old-operation', outcomes: receipt().outcomes, records: [{ packageId: 'package-one', revisionId: 'old-revision', nameAtWrite: 'old-review-name', physicalPath: '/fixture/old-review-name', scope: 'user', project: null, selectedProfiles: [{ agentId: 'codex', variantId: 'default', rootId: 'native' }] }], warnings: [] }];
      case 'list_recoveries': return window.pendingRecovery?[{operationId:'interrupted-operation',kind:'distribution',state:'recovery_required',reason:null,paths:['/fixture/interrupted']}]:[];
      case 'preview_restore': return preview(request);
      case 'commit_restore': if(window.failRestore)throw {code:'restore_failed'};return receipt();
      default: throw { code: 'unknown_command' };
    }
  } };
}
