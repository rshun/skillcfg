import type { LibraryRecord, Preview, Profile, SettingsView } from './types';
// Browser-only demonstrations never read or write the host filesystem.
const profiles: Profile[] = ['codex', 'claude-code', 'antigravity', 'grok'].map((id, i) => ({ id, displayName: ['Codex', 'Claude Code', 'Antigravity', 'Grok Build'][i], revision: `demo-${i}`, source: 'builtin', state: 'builtin', variants: ['default'], currentVariant: 'default', roots: [{ agent: id, rootId: 'native', variant: 'default', path: `/demo/${id}/skills`, actualPath: `/demo/${id}/skills`, rootWasLink: false, writable: true, priority: 100 }], issues: [] }));
const settings: SettingsView = { settings: { version: 1, libraryPath: '/demo/library', backupPath: '/demo/backups', defaultScope: 'user', lastProjectDir: null, agentVariants: {} }, revision: 'demo-settings', warnings: [] };
let records: LibraryRecord[] = ['code-review', 'incident-response', 'writing-plans'].map((name, i) => ({ id: `demo-${i}`, name, displayName: name, description: ['逐项审阅变更，核对边界与回归证据。', '保留现场，整理线索与处置步骤。', '将目标整理为可验证的实施计划。'][i], sourceKind: 'demo', sourceAgent: null, sourcePath: `/demo/source/${name}`, currentRevision: 'demo-revision', importedAt: 1793680000 }));
let last: { kind: string; preview: Preview } | null = null;
let serial = 0;
export async function mockCall<T>(command: string, request: Record<string, unknown>): Promise<T> {
  let value: unknown;
  if (command.startsWith('preview_') && command !== 'preview_agent_profile') {
    const preview: Preview = { previewId: `demo-preview-${++serial}`, expiresAt: Date.now()/1000 + 600, blocked: false, backupLocation: '/demo/backups', warnings: ['此为演示预览，不读取或修改本机文件。'], groups: [{ targetKey: 'demo-target', path: '/demo/target/code-review', disposition: 'new', selectedAgents: ((request.selection as {agent:string}[] | undefined) || []).map(p => p.agent), possibleReaders: [], files: [{ change:'added',beforeKind:null,afterKind:'file',beforeExecutable:null,afterExecutable:false,path: 'SKILL.md', before: null, after: '---\nname: code-review\ndescription: Review changes.\n---\n', beforeBytes: 0, afterBytes: 60, binary: false, truncated: false }], reason: null }] };
    last = { kind: command.slice(8), preview }; value = preview;
  } else if (command.startsWith('commit_')) {
    if (command === 'commit_agent_profile') { value = { operationId: 'demo-profile', backupLocation: '/demo/backups', outcomes: [] }; }
    else {
      if (!last || request.previewId !== last.preview.previewId || command !== `commit_${last.kind}`) throw { code: 'preview_consumed' };
      if (last.kind === 'import') records = [...records, { ...records[0], id: `demo-import-${++serial}`, name: 'imported-skill', displayName: 'imported-skill' }];
      value = { operationId: `demo-operation-${++serial}`, backupLocation: '/demo/backups', outcomes: [{ path: last.preview.groups[0].path, outcome: 'written', reason: null }] }; last = null;
    }
  } else switch (command) {
    case 'app_info': value = { version: '0.1.0', platform: 'linux', mode: 'demo' }; break;
    case 'load_settings': value = structuredClone(settings); break;
    case 'save_settings': settings.settings = request.settings as SettingsView['settings']; settings.revision = `demo-settings-${++serial}`; value = structuredClone(settings); break;
    case 'load_library': value = { firstUse: false, records }; break;
    case 'list_agent_profiles': value = profiles; break;
    case 'detect_agents': value = profiles.map(p => ({ agentId: p.id, installed: false, issues: [] })); break;
    case 'library_statuses': value = records.flatMap((r, i) => profiles.map((p,n) => ({ packageId: r.id, agentId: p.id, state: i===0&&n===0?'equal':i===1&&n===0?'different':'missing', path: `/demo/${p.id}/skills/${r.name}` }))); break;
    case 'read_package': { const r=records.find(r=>r.id===request.packageId); if (!r) throw { code:'unknown_package' }; value = { packageId: r.id, revisionId: r.currentRevision, document: `---\nname: ${r.name}\ndescription: ${r.description}\n---\n\n先核对输入与上下文，再按步骤完成任务。\n`, editToken: 'demo-edit', files: ['SKILL.md', 'references/checklist.md'], issues: [] }; break; }
    case 'list_agent_skills': value = { agentId: request.agentId, installed: false, roots: profiles.find(p=>p.id===request.agentId)?.roots || [], items: [], errors: [] }; break;
    case 'choose_directory': value = '/demo/source'; break;
    case 'scan_import': value = { scanId: 'demo-scan', items: [{ itemId: 'demo-item', name: 'imported-skill', sourcePath: request.source, actualPath: request.source, issues: [] }] }; break;
    case 'save_skill_document': value = { operationId: 'demo-edit', backupLocation: '/demo/backups', outcomes: [] }; break;
    case 'preview_agent_profile': value = { previewId: 'demo-profile', profileId: 'custom.example', resolvedRoots: ['/demo/.example/skills'], probePaths: [], variants: ['default'], blocked: false, warnings: ['演示环境不会保存档案。'] }; break;
    case 'set_agent_profile_enabled': value = { operationId: 'demo-profile', backupLocation: '/demo/backups', outcomes: [] }; break;
    case 'read_agent_profile': throw { code: 'demo_profile_readonly' };
    case 'list_history': case 'list_recoveries': value = []; break;
    case 'cancel_preview': if (last?.preview.previewId === request.previewId) last = null; value = null; break;
    case 'set_context': last = null; value = null; break;
    default: throw { code: 'unknown_command' };
  }
  return structuredClone(value) as T;
}
