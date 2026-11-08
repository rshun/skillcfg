import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type {
  AppInfo,
  Detection,
  HistoryItem,
  ImportChoice,
  LibraryView,
  PackageDocument,
  Profile,
  Recovery,
  SettingsView,
  TargetStatus,
} from './types';
import { call } from './api';
import { errorCode, SessionController } from './session';
import { selectionFromCells } from './selection';
import { readInitialAccent, readInitialTheme, type AccentId, type ThemeMode } from './theme';
import { AppShell, type Page } from './components/AppShell';
import { LibrarySwitchDialog } from './components/LibrarySwitchDialog';
import { Icon } from './components/Icon';
import { Library } from './pages/Library';
import { Agents } from './pages/Agents';
import { History } from './pages/History';
import { Settings } from './pages/Settings';
import { Import } from './pages/Import';
import { SkillEditor } from './pages/SkillEditor';
import { AgentProfileEditor } from './pages/AgentProfileEditor';
import { MatrixDialog } from './pages/Matrix';
import { Preview } from './pages/Preview';
import { message } from './messages';
import './styles.css';

type Modal =
  | { kind: 'import'; source: string }
  | { kind: 'edit'; packageId: string }
  | { kind: 'profile'; profile: Profile | null }
  | { kind: 'switch-library' }
  | { kind: 'matrix' }
  | null;

type LastRequest = { kind: string; args: Record<string, unknown>; source?: string };

function profileSignature(profiles: Profile[]) {
  return JSON.stringify(
    profiles
      .map(({ revision, ...p }) => ({ ...p, roots: p.roots.map(({ exists, ...r }) => r) }))
      .sort((a, b) => (a.id || '').localeCompare(b.id || '')),
  );
}

export default function App() {
  const [session] = useState(() => new SessionController(call));
  const state = useSyncExternalStore(session.subscribe, session.snapshot);

  const [page, setPage] = useState<Page>('library');
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [library, setLibrary] = useState<LibraryView | null>(null);
  const [libraryError, setLibraryError] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [profilesError, setProfilesError] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<TargetStatus[]>([]);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [detections, setDetections] = useState<Detection[]>([]);
  const [detectionError, setDetectionError] = useState<string | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [recoveries, setRecoveries] = useState<Recovery[]>([]);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);

  const [selectedId, setSelectedId] = useState('');
  const [document, setDocument] = useState<PackageDocument | null>(null);
  const [documentError, setDocumentError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [scopeChanging, setScopeChanging] = useState(false);
  const [modal, setModal] = useState<Modal>(null);
  const [announcement, setAnnouncement] = useState('');
  const [decisions, setDecisions] = useState<Record<string, string>>({});
  const [theme, setTheme] = useState<ThemeMode>(readInitialTheme);
  const [accent, setAccent] = useState<AccentId>(readInitialAccent);

  const readEpoch = useRef(0);
  const navEpoch = useRef(0);
  const profileRevision = useRef('');
  const lastRequest = useRef<LastRequest | null>(null);
  const scopeLock = useRef(false);
  const leave = useRef<() => Promise<boolean>>(async () => true);

  const registerGuard = useCallback((guard: (() => Promise<boolean>) | null) => {
    leave.current = guard || (async () => true);
  }, []);

  const busy = state.phase === 'confirming' || scopeChanging;

  useEffect(() => {
    globalThis.document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('skill-hub-theme', theme);
    } catch {
      /* Theme persistence is optional. */
    }
  }, [theme]);

  useEffect(() => {
    globalThis.document.documentElement.dataset.accent = accent;
    try {
      localStorage.setItem('skill-hub-accent', accent);
    } catch {
      /* Accent persistence is optional. */
    }
  }, [accent]);

  const refresh = useCallback(async () => {
    const epoch = ++readEpoch.current;
    const generation = session.state.generation;
    setRefreshing(true);
    setStatuses([]);
    setDetections([]);
    setStatusError(null);
    setDetectionError(null);

    const query = async <T,>(
      command: string,
      store: (value: T) => void,
      fail: (code: string | null) => void,
    ) => {
      try {
        const value = await session.read<T>(command);
        if (value !== null && epoch === readEpoch.current && generation === session.state.generation) {
          store(value);
          fail(null);
        }
      } catch (e) {
        if (epoch === readEpoch.current && generation === session.state.generation) {
          fail(errorCode(e));
        }
      }
    };

    await Promise.all([
      query<SettingsView>('load_settings', setSettings, setSettingsError),
      query<LibraryView>(
        'load_library',
        v => {
          setLibrary(v);
          setSelectedId(old => (v.records.some(r => r.id === old) ? old : v.records[0]?.id || ''));
        },
        setLibraryError,
      ),
      query<Profile[]>(
        'list_agent_profiles',
        v => {
          const signature = profileSignature(v);
          if (profileRevision.current && profileRevision.current !== signature) void session.clear();
          profileRevision.current = signature;
          setProfiles(v);
        },
        code => {
          setProfilesError(code);
          if (code) setProfiles([]);
        },
      ),
      query<TargetStatus[]>('library_statuses', setStatuses, setStatusError),
      query<Detection[]>('detect_agents', setDetections, setDetectionError),
      query<HistoryItem[]>('list_history', setHistory, setHistoryError),
      query<Recovery[]>('list_recoveries', setRecoveries, setRecoveryError),
    ]);

    if (epoch === readEpoch.current && generation === session.state.generation) {
      setRefreshing(false);
      setReady(true);
    }
  }, [session]);

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const data = await call<AppInfo>('app_info');
        if (active) setInfo(data);
      } catch {
        if (active) setAnnouncement('无法连接本机服务，请重新打开桌面窗口。');
      }
      try {
        const initial = await call<SettingsView>('load_settings');
        if (!active) return;
        setSettings(initial);
        await session.context({
          libraryPath: initial.settings.libraryPath,
          scope: 'user',
          projectRoot: null,
        });
        if (initial.settings.defaultScope === 'project' && initial.settings.lastProjectDir) {
          await session.context({ scope: 'project', projectRoot: initial.settings.lastProjectDir });
        }
      } catch (e) {
        if (active) setSettingsError(errorCode(e));
      }
      if (active) await refresh();
    })();
    return () => {
      active = false;
      ++readEpoch.current;
    };
  }, [refresh, session]);

  useEffect(() => {
    let active = true;
    setDocument(null);
    setDocumentError(null);
    if (selectedId && !libraryError) {
      void session
        .read<PackageDocument>('read_package', { packageId: selectedId })
        .then(value => {
          if (active && value) setDocument(value);
        })
        .catch(e => {
          if (active) setDocumentError(errorCode(e));
        });
    }
    return () => {
      active = false;
    };
  }, [selectedId, library?.records, libraryError, state.generation, session]);

  useEffect(() => {
    const timer = setInterval(() => session.expire(), 1000);
    return () => clearInterval(timer);
  }, [session]);

  useEffect(() => {
    if (state.phase === 'result') void refresh();
  }, [state.phase, state.receipt?.operationId, refresh]);

  async function navigate(next: Page) {
    if (busy || !(await leave.current())) return;
    const epoch = ++navEpoch.current;
    await session.cancel();
    if (epoch === navEpoch.current) {
      setPage(next);
      setAnnouncement('');
    }
  }

  async function scope(nextScope: 'user' | 'project') {
    if (busy || scopeLock.current) return;
    scopeLock.current = true;
    setScopeChanging(true);
    try {
      if (!(await leave.current())) return;
      const changed =
        nextScope === 'project'
          ? await session.pickProject()
          : await session.context({ scope: 'user', projectRoot: null });
      if (changed) await refresh();
    } catch (e) {
      setAnnouncement(message(errorCode(e)));
    } finally {
      scopeLock.current = false;
      setScopeChanging(false);
    }
  }

  async function configurationChanged() {
    const view = await call<SettingsView>('load_settings');
    await session.context({ libraryPath: view.settings.libraryPath });
    setSettings(view);
    await refresh();
    setAnnouncement('配置已保存，本次选择与旧预览已清空。');
  }

  async function librarySaved() {
    await session.cancel();
    await refresh();
    setAnnouncement('已保存到库，尚未分发。');
  }

  async function openTask(next: Modal) {
    if (busy || !(await leave.current())) return;
    if (next?.kind !== 'matrix') await session.cancel();
    setModal(next);
    setAnnouncement('');
  }

  async function startPreview(kind: string, args: Record<string, unknown>, source?: string) {
    setModal(null);
    lastRequest.current = { kind, args, source };
    if (kind !== 'distribution') setDecisions({});
    await session.prepare(kind, args);
  }

  function distribute() {
    setDecisions({});
    void startPreview('distribution', {
      selection: selectionFromCells(state.selection),
      conflictDecisions: {},
    });
  }

  function changeDecision(key: string, value: string) {
    const last = lastRequest.current;
    if (!last || last.kind !== 'distribution') return;
    const next = { ...decisions, [key]: value };
    setDecisions(next);
    void startPreview('distribution', { ...last.args, conflictDecisions: next });
  }

  function repreview() {
    const last = lastRequest.current;
    if (!last) return;
    if (last.kind === 'import') {
      void session.cancel().then(() => setModal({ kind: 'import', source: last.source || '' }));
    } else {
      void session.prepare(last.kind, last.args);
    }
  }

  const selectedRecord = modal?.kind === 'edit' ? library?.records.find(r => r.id === modal.packageId) : null;
  const mismatch = !!settings && !!state.libraryPath && settings.settings.libraryPath !== state.libraryPath;
  const pairs = selectionFromCells(state.selection);
  const selectedSkillCount = new Set(pairs.map(p => p.packageId)).size;
  const selectedAgentCount = new Set(pairs.map(p => p.agent)).size;

  return (
    <AppShell
      page={page}
      onPage={p => void navigate(p)}
      info={info}
      state={scopeChanging ? { ...state, phase: 'confirming' } : state}
      skillCount={library?.records.length ?? 0}
      refreshing={refreshing}
      theme={theme}
      accent={accent}
      onThemeChange={setTheme}
      onToggleTheme={() => setTheme(prev => (prev === 'dark' ? 'light' : 'dark'))}
      onAccentChange={setAccent}
      onScope={s => void scope(s)}
      onSwitchLibrary={() =>
        settingsError ? void navigate('settings') : void openTask({ kind: 'switch-library' })
      }
      onRefresh={() => void refresh()}
    >
      <div className="content main-content" data-ready={ready}>
        {announcement && (
          <div className="notice error-block" role="status">
            <span>{announcement}</span>
            <button
              type="button"
              className="button quiet icon-button small"
              aria-label="关闭提示"
              onClick={() => setAnnouncement('')}
            >
              <Icon name="close" size={14} />
            </button>
          </div>
        )}
        {settingsError && (
          <div className="notice error error-block" role="alert">
            {message(settingsError)}
          </div>
        )}
        {state.error && state.phase === 'idle' && (
          <div className="notice error error-block" role="alert">
            {message(state.error)}
          </div>
        )}
        {mismatch && (
          <div className="notice warning error-block">
            <span>配置库位置已在外部改变，请重新载入工作台后继续。</span>
            <button className="button small" type="button" onClick={() => void configurationChanged()}>
              重新载入
            </button>
          </div>
        )}
        {recoveries.length > 0 && page !== 'history' && (
          <div className="notice warning error-block">
            <Icon name="warning" size={17} />
            <span>有 {recoveries.length} 项事务需要处理。</span>
            <button type="button" className="text-button" onClick={() => void navigate('history')}>
              查看待恢复任务
            </button>
          </div>
        )}

        {page === 'library' && (
          <Library
            library={library}
            error={libraryError}
            statusError={statusError}
            profiles={profiles}
            statuses={statuses}
            selectedId={selectedId}
            onSelect={setSelectedId}
            document={document}
            documentError={documentError}
            state={state}
            onTarget={(key, value) => session.select(key, value)}
            onImport={() => void openTask({ kind: 'import', source: '' })}
            onAgents={() => void navigate('agents')}
            onEdit={() => {
              if (selectedId) void openTask({ kind: 'edit', packageId: selectedId });
            }}
            onSwitchLibrary={() =>
              settingsError ? void navigate('settings') : void openTask({ kind: 'switch-library' })
            }
            onOpenMatrix={() => void openTask({ kind: 'matrix' })}
            onRefresh={() => void refresh()}
            history={history}
          />
        )}

        {page === 'agents' && (
          <Agents
            profiles={profiles}
            detections={detections}
            detectionError={detectionError}
            error={profilesError}
            session={session}
            busy={busy}
            onProfile={p => void openTask({ kind: 'profile', profile: p })}
            onImport={source => void openTask({ kind: 'import', source })}
            onChanged={configurationChanged}
          />
        )}

        {page === 'history' && (
          <History
            items={history}
            recoveries={recoveries}
            error={historyError}
            recoveryError={recoveryError}
            busy={busy}
            onRestore={id => void startPreview('restore', { operationId: id })}
            onRefresh={() => void refresh()}
          />
        )}

        {page === 'settings' && (
          <Settings
            key={state.generation}
            call={call}
            theme={theme}
            accent={accent}
            onThemeChange={setTheme}
            onAccentChange={setAccent}
            onSaved={configurationChanged}
            onHistory={() => void navigate('history')}
            registerGuard={registerGuard}
          />
        )}
      </div>

      {page === 'library' && (
        <footer className={`selection-bar${state.selection.length > 0 ? ' active' : ''}`} aria-label="本次分发选择">
          <span className="selection-mark">
            <Icon name="check" size={16} />
          </span>
          <div className="selection-copy">
            <p className="selection-count">
              <b>{state.selection.length}</b> 个目标配对
              {state.selection.length > 0 && (
                <span className="selection-meta">
                  {' '}
                  · {selectedSkillCount} 个 Skill / {selectedAgentCount} 个 Agent
                </span>
              )}
            </p>
            <p className="selection-hint">仅记录本次选择，预览逐文件影响后手动应用</p>
          </div>
          <div className="spacer" />
          <div className="row selection-actions">
            <button
              type="button"
              className="button quiet"
              disabled={!state.selection.length || busy}
              onClick={() => void session.clear()}
            >
              清空
            </button>
            <button
              type="button"
              className="button primary"
              data-action="preview-distribution"
              disabled={
                !state.selection.length ||
                busy ||
                refreshing ||
                mismatch ||
                !!settingsError ||
                !!libraryError ||
                !!state.error
              }
              onClick={distribute}
            >
              预览变更
              <Icon name="arrow" size={16} />
            </button>
          </div>
        </footer>
      )}

      {modal?.kind === 'import' && (
        <Import
          source={modal.source}
          records={library?.records || []}
          call={call}
          onClose={() => setModal(null)}
          onPreview={(scanId: string, choices: ImportChoice[], source: string) =>
            startPreview('import', { scanId, choices }, source)
          }
        />
      )}

      {modal?.kind === 'edit' && selectedRecord && (
        <SkillEditor
          record={selectedRecord}
          call={call}
          onClose={() => setModal(null)}
          onSaved={librarySaved}
        />
      )}

      {modal?.kind === 'profile' && (
        <AgentProfileEditor
          profile={modal.profile}
          info={info}
          scope={state.scope}
          projectRoot={state.projectRoot}
          call={call}
          onClose={() => setModal(null)}
          onSaved={configurationChanged}
        />
      )}

      {modal?.kind === 'switch-library' && (
        <LibrarySwitchDialog
          initialPath={state.libraryPath}
          call={call}
          onClose={() => setModal(null)}
          onSwitched={configurationChanged}
        />
      )}

      {modal?.kind === 'matrix' && (
        <MatrixDialog
          records={library?.records || []}
          profiles={profiles}
          statuses={statuses}
          statusError={statusError}
          state={state}
          onTarget={(key, value) => session.select(key, value)}
          onClear={() => void session.clear()}
          onClose={() => setModal(null)}
          onPreview={distribute}
        />
      )}

      {state.kind && state.phase !== 'idle' && (
        <Preview
          state={state}
          session={session}
          profiles={profiles}
          decisions={decisions}
          onDecision={changeDecision}
          onRepreview={repreview}
        />
      )}
    </AppShell>
  );
}
