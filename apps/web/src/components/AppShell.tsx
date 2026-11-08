import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { AppInfo } from '../types';
import type { SessionState } from '../session';
import { ACCENT_PRESETS, folderName, libraryLabel, type AccentId, type ThemeMode } from '../theme';
import { Icon, type IconName } from './Icon';

export type Page = 'library' | 'agents' | 'history' | 'settings';

const PRIMARY_NAV: { id: Page; title: string; icon: IconName; hint: string }[] = [
  { id: 'library', title: '我的 Skills', icon: 'library', hint: '管理完整包与分发' },
  { id: 'agents', title: 'Agents', icon: 'agents', hint: '查看目录与接入档案' },
  { id: 'history', title: '变更历史', icon: 'history', hint: '逐组回执与备份恢复' },
];

export function AppShell({
  page,
  onPage,
  info,
  state,
  skillCount,
  refreshing,
  theme,
  accent,
  onThemeChange,
  onToggleTheme,
  onAccentChange,
  onScope,
  onSwitchLibrary,
  onRefresh,
  children,
}: {
  page: Page;
  onPage: (p: Page) => void;
  info: AppInfo | null;
  state: SessionState;
  skillCount: number;
  refreshing: boolean;
  theme: ThemeMode;
  accent: AccentId;
  onThemeChange: (next: ThemeMode) => void;
  onToggleTheme: () => void;
  onAccentChange: (next: AccentId) => void;
  onScope: (scope: 'user' | 'project') => void;
  onSwitchLibrary: () => void;
  onRefresh: () => void;
  children: ReactNode;
}) {
  const busy = state.phase === 'confirming';
  const [paletteOpen, setPaletteOpen] = useState(false);
  const paletteRef = useRef<HTMLDivElement | null>(null);
  const currentAccent = ACCENT_PRESETS.find(item => item.id === accent) ?? ACCENT_PRESETS[0];
  const currentTitle = PRIMARY_NAV.find(n => n.id === page)?.title ?? '软件设置';
  const currentLibrary = libraryLabel(state.libraryPath);

  useEffect(() => {
    if (!paletteOpen) return;
    const onPointerDown = (event: MouseEvent) => {
      if (paletteRef.current && !paletteRef.current.contains(event.target as Node)) {
        setPaletteOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setPaletteOpen(false);
    };
    document.addEventListener('mousedown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [paletteOpen]);

  return (
    <div className="shell app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <Icon name="library" size={21} />
          </span>
          <div className="brand-text">
            Skill Hub
            <small>统一管理 Skill 包</small>
          </div>
        </div>

        <button
          type="button"
          className="library-switch"
          data-action="switch-library"
          aria-label="切换配置库"
          title={state.libraryPath || '切换配置库目录'}
          disabled={busy}
          onClick={() => {
            setPaletteOpen(false);
            onSwitchLibrary();
          }}
        >
          <Icon name="folder" size={18} />
          <span>
            <small>当前配置库</small>
            <b>{currentLibrary}</b>
          </span>
          <Icon name="chevron" size={15} />
        </button>

        <nav className="nav" aria-label="主导航">
          {PRIMARY_NAV.map(n => (
            <button
              type="button"
              className={`nav-item${page === n.id ? ' active' : ''}`}
              key={n.id}
              aria-label={n.title}
              data-page={n.id}
              aria-current={page === n.id ? 'page' : undefined}
              disabled={busy}
              onClick={() => {
                setPaletteOpen(false);
                onPage(n.id);
              }}
            >
              <Icon name={n.icon} size={18} />
              <span className="nav-text">
                {n.title}
                <small>{n.hint}</small>
              </span>
              {n.id === 'library' && skillCount > 0 && <span className="count">{skillCount}</span>}
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <button
            type="button"
            className={`nav-item${page === 'settings' ? ' active' : ''}`}
            aria-label="软件设置"
            data-page="settings"
            aria-current={page === 'settings' ? 'page' : undefined}
            disabled={busy}
            onClick={() => {
              setPaletteOpen(false);
              onPage('settings');
            }}
          >
            <Icon name="settings" size={18} />
            <span className="nav-text">软件设置</span>
          </button>
          <div className="version-line">
            <span className="status-dot" />
            <span className="version">{info ? `v${info.version}` : '正在初始化…'}</span>
            <span className="manual-tag">仅手动应用</span>
          </div>
        </div>
      </aside>

      <main className="main" aria-busy={busy}>
        <header className="topbar">
          <span className="breadcrumb">
            <span>工作空间</span>
            <span>/</span>
            <strong>{currentTitle}</strong>
          </span>
          <div className="spacer" />

          <div className="workspace-context">
            <div className="scope" role="group" aria-label="配置范围">
              <button
                type="button"
                data-scope="user"
                aria-pressed={state.scope === 'user'}
                onClick={() => onScope('user')}
                disabled={busy}
              >
                全局配置
              </button>
              <button
                type="button"
                data-scope="project"
                aria-pressed={state.scope === 'project'}
                onClick={() => onScope('project')}
                disabled={busy}
              >
                项目配置
              </button>
            </div>
            {state.scope === 'project' && (
              <button
                type="button"
                className={`button project-button${!state.projectRoot ? ' needs-project' : ''}`}
                aria-label="更换项目目录"
                title={state.projectRoot || '选择项目目录'}
                onClick={() => onScope('project')}
                disabled={busy}
              >
                <Icon name="folder" size={15} />
                <span>{folderName(state.projectRoot) || '选择项目目录'}</span>
              </button>
            )}
          </div>

          {info && info.mode !== 'desktop' && (
            <span className="pill warn mode-pill" title="操作仅保存在当前页面，不读取本机配置">
              {info.mode === 'fixture' ? '测试环境' : '演示数据'}
            </span>
          )}

          <div className="theme-picker" ref={paletteRef}>
            <button
              type="button"
              className={`button quiet icon-button palette-trigger${paletteOpen ? ' active' : ''}`}
              aria-label="切换主题配色"
              title={`主题配色：${currentAccent.label}`}
              aria-expanded={paletteOpen}
              onClick={() => setPaletteOpen(prev => !prev)}
            >
              <Icon name="palette" size={18} />
              <span className="palette-dot" style={{ background: currentAccent.swatch }} aria-hidden="true" />
            </button>
            {paletteOpen && (
              <div className="theme-popover" role="dialog" aria-label="界面主题配色">
                <div className="theme-popover-section">
                  <span className="theme-popover-title">外观模式</span>
                  <div className="theme-mode-switch">
                    <button
                      type="button"
                      className={`theme-mode-btn${theme === 'light' ? ' active' : ''}`}
                      onClick={() => onThemeChange('light')}
                    >
                      <Icon name="sun" size={15} />
                      浅色
                    </button>
                    <button
                      type="button"
                      className={`theme-mode-btn${theme === 'dark' ? ' active' : ''}`}
                      onClick={() => onThemeChange('dark')}
                    >
                      <Icon name="moon" size={15} />
                      深色
                    </button>
                  </div>
                </div>
                <div className="theme-popover-section">
                  <span className="theme-popover-title">主题色调</span>
                  <div className="accent-grid">
                    {ACCENT_PRESETS.map(item => (
                      <button
                        key={item.id}
                        type="button"
                        data-accent-choice={item.id}
                        className={`accent-chip${accent === item.id ? ' active' : ''}`}
                        onClick={() => onAccentChange(item.id)}
                        aria-pressed={accent === item.id}
                      >
                        <span className="accent-swatch" style={{ background: item.swatch }} aria-hidden="true" />
                        <span>{item.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>

          <button
            type="button"
            className="button quiet icon-button"
            data-action="theme"
            aria-label={theme === 'dark' ? '切换浅色主题' : '切换深色主题'}
            title={theme === 'dark' ? '切换到浅色外观' : '切换到深色外观'}
            onClick={onToggleTheme}
          >
            <Icon name={theme === 'dark' ? 'sun' : 'moon'} size={18} />
          </button>

          <button
            type="button"
            className="button quiet icon-button"
            aria-label="刷新配置与状态"
            title="刷新配置与状态"
            disabled={busy || refreshing}
            onClick={onRefresh}
          >
            <Icon name="refresh" size={18} />
          </button>
        </header>

        {state.scope === 'project' && state.projectRoot && (
          <div className="project-bar">
            <Icon name="folder" size={14} />
            <span>当前项目：</span>
            <code className="path">{state.projectRoot}</code>
          </div>
        )}

        {children}
      </main>
    </div>
  );
}
