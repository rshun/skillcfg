import { useEffect, useState } from 'react';
import type { Settings as SettingsData, SettingsView, Transport } from '../types';
import { ACCENT_PRESETS, type AccentId, type ThemeMode } from '../theme';
import { useDiscardConfirmation } from '../components/DiscardConfirmation';
import { LocationForm } from '../components/LocationForm';
import { Icon } from '../components/Icon';
import { errorCode } from '../session';
import { message } from '../messages';

export function Settings({
  call,
  theme,
  accent,
  onThemeChange,
  onAccentChange,
  onSaved,
  onHistory,
  registerGuard,
}: {
  call: Transport;
  theme: ThemeMode;
  accent: AccentId;
  onThemeChange: (next: ThemeMode) => void;
  onAccentChange: (next: AccentId) => void;
  onSaved: () => Promise<void>;
  onHistory: () => void;
  registerGuard: (guard: (() => Promise<boolean>) | null) => void;
}) {
  const ask = useDiscardConfirmation();
  const [view, setView] = useState<SettingsView | null>(null);
  const [draft, setDraft] = useState<SettingsData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let active = true;
    void call<SettingsView>('load_settings')
      .then(v => {
        if (active) {
          setView(v);
          setDraft(v.settings);
        }
      })
      .catch(e => {
        if (active) setError(errorCode(e));
      });
    return () => {
      active = false;
    };
  }, [call]);

  async function pick(key: 'libraryPath' | 'backupPath' | 'lastProjectDir') {
    if (!draft) return;
    try {
      const p = await call<string | null>('choose_directory', { initial: draft[key] || null });
      if (p) {
        setDraft({ ...draft, [key]: p });
        setSaved(false);
      }
    } catch (e) {
      setError(errorCode(e));
    }
  }

  function update<K extends keyof SettingsData>(key: K, value: SettingsData[K]) {
    if (draft) {
      setDraft({ ...draft, [key]: value });
      setSaved(false);
    }
  }

  async function save() {
    if (!view || !draft) return;
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      const fresh = await call<SettingsView>('load_settings');
      if (JSON.stringify(fresh.settings) !== JSON.stringify(view.settings)) {
        throw { code: 'settings_changed' };
      }
      const v = await call<SettingsView>('save_settings', {
        expectedRevision: fresh.revision,
        settings: draft,
      });
      setView(v);
      setDraft(v.settings);
      setSaved(true);
      await onSaved();
    } catch (e) {
      setError(errorCode(e));
    } finally {
      setBusy(false);
    }
  }

  const dirty = JSON.stringify(view?.settings) !== JSON.stringify(draft);
  useEffect(() => {
    registerGuard(async () => !busy && (!dirty || (await ask('放弃尚未保存的设置修改？'))));
    return () => registerGuard(null);
  }, [dirty, busy, ask, registerGuard]);

  return (
    <section className="settings-page">
      <header className="page-heading">
        <div>
          <h1>软件设置</h1>
          <p className="sub">指定默认配置库、中心备份位置与界面外观，与当前会话范围相互独立。</p>
        </div>
      </header>

      <section className="panel settings-panel">
        <div className="panel-head">启动与存储</div>
        <div className="panel-body stack">
          {error && (
            <div className="notice error" role="alert">
              {message(error)}
            </div>
          )}
          {saved && (
            <p className="notice success" role="status">
              设置已保存，本次配对和旧预览已清空。
            </p>
          )}
          {draft ? (
            <>
              <LocationForm
                label="配置库目录"
                value={draft.libraryPath}
                onChange={v => update('libraryPath', v)}
                onPick={() => void pick('libraryPath')}
                disabled={busy}
              />
              <p className="form-hint">保存后切换当前库。原配置库与其历史备份完整保留。</p>

              <LocationForm
                label="后续备份目录"
                value={draft.backupPath}
                onChange={v => update('backupPath', v)}
                onPick={() => void pick('backupPath')}
                disabled={busy}
              />
              <p className="form-hint">仅影响后续操作。已有备份仍可从变更历史中查看与恢复。</p>

              <div className="settings-divider" />

              <label className="field">
                <span>启动时默认范围</span>
                <select
                  value={draft.defaultScope}
                  disabled={busy}
                  onChange={e => update('defaultScope', e.target.value as 'user' | 'project')}
                >
                  <option value="user">全局配置 · 当前用户跨项目</option>
                  <option value="project">项目配置 · 上次选择的项目目录</option>
                </select>
              </label>

              <LocationForm
                label="最近项目目录"
                value={draft.lastProjectDir || ''}
                onChange={v => update('lastProjectDir', v || null)}
                onPick={() => void pick('lastProjectDir')}
                disabled={busy}
              />

              <div className="detail-note">
                <Icon name="shield" size={16} />
                <span>仅手动应用。导入和编辑只保存到配置库，所有分发与恢复都需先预览再由你确认。</span>
              </div>

              <div className="row spread">
                <p className="form-hint">文件内容通过历史备份恢复，回退软件版本不能代替数据恢复。</p>
                <button
                  type="button"
                  className="button primary"
                  data-action="save-settings"
                  disabled={busy || !dirty}
                  onClick={() => void save()}
                >
                  {busy ? '正在保存…' : '保存设置'}
                </button>
              </div>
            </>
          ) : !error ? (
            <p className="loading">正在读取设置…</p>
          ) : (
            <>
              <p className="muted">修复前保留当前设置文件，不创建空白设置覆盖它。</p>
              <div>
                <button type="button" className="button" onClick={onHistory}>
                  查看历史备份
                </button>
              </div>
            </>
          )}
        </div>
      </section>

      <section className="panel settings-panel">
        <div className="panel-head">外观与主题色</div>
        <div className="panel-body stack">
          <div className="field">
            <span>明暗外观</span>
            <div className="theme-mode-switch settings-mode-switch">
              <button
                type="button"
                className={`theme-mode-btn${theme === 'light' ? ' active' : ''}`}
                onClick={() => onThemeChange('light')}
              >
                <Icon name="sun" size={15} />
                浅色外观
              </button>
              <button
                type="button"
                className={`theme-mode-btn${theme === 'dark' ? ' active' : ''}`}
                onClick={() => onThemeChange('dark')}
              >
                <Icon name="moon" size={15} />
                深色外观
              </button>
            </div>
          </div>

          <div className="field">
            <span>主题色调</span>
            <div className="settings-accent-grid">
              {ACCENT_PRESETS.map(item => (
                <button
                  key={item.id}
                  type="button"
                  className={`settings-accent-card${accent === item.id ? ' active' : ''}`}
                  onClick={() => onAccentChange(item.id)}
                  aria-pressed={accent === item.id}
                >
                  <span className="accent-swatch large" style={{ background: item.swatch }} aria-hidden="true" />
                  <span>
                    <b>{item.label}</b>
                    <small>{item.desc}</small>
                  </span>
                </button>
              ))}
            </div>
          </div>

          <p className="form-hint">切换后立即生效并自动记住，也可随时通过右上角调色盘按钮快速切换。</p>
        </div>
      </section>
    </section>
  );
}
