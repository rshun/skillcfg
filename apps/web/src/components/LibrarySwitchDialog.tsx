import { useEffect, useState } from 'react';
import type { SettingsView, Transport } from '../types';
import { Dialog } from './Dialog';
import { LocationForm } from './LocationForm';
import { Icon } from './Icon';
import { errorCode } from '../session';
import { message } from '../messages';

export function LibrarySwitchDialog({
  initialPath,
  call,
  onClose,
  onSwitched,
}: {
  initialPath: string;
  call: Transport;
  onClose: () => void;
  onSwitched: () => Promise<void>;
}) {
  const [view, setView] = useState<SettingsView | null>(null);
  const [path, setPath] = useState(initialPath);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void call<SettingsView>('load_settings')
      .then(v => {
        if (active) {
          setView(v);
          if (!initialPath) setPath(v.settings.libraryPath);
        }
      })
      .catch(e => {
        if (active) setError(errorCode(e));
      });
    return () => {
      active = false;
    };
  }, [call, initialPath]);

  async function pick() {
    setError(null);
    try {
      const chosen = await call<string | null>('choose_directory', { initial: path || null });
      if (chosen) setPath(chosen);
    } catch (e) {
      setError(errorCode(e));
    }
  }

  async function submit() {
    const nextPath = path.trim();
    if (!view || !nextPath) return;
    setBusy(true);
    setError(null);
    try {
      const fresh = await call<SettingsView>('load_settings');
      if (JSON.stringify(fresh.settings) !== JSON.stringify(view.settings)) {
        throw { code: 'settings_changed' };
      }
      await call<SettingsView>('save_settings', {
        expectedRevision: fresh.revision,
        settings: { ...fresh.settings, libraryPath: nextPath },
      });
      await onSwitched();
      onClose();
    } catch (e) {
      setError(errorCode(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="管理配置库" locked={busy} onClose={onClose}>
      <div className="dialog-body">
        <p className="muted">
          配置库是你统一维护完整 Skill 包的本地目录，与各 Agent 原生加载目录相互独立。切换配置库不会自动修改任何 Agent。
        </p>
        {error && (
          <div className="notice error" role="alert">
            {message(error)}
          </div>
        )}
        <LocationForm
          label="配置库绝对目录"
          value={path}
          onChange={setPath}
          onPick={() => void pick()}
          disabled={busy || !view}
        />
        <div className="notice">
          <Icon name="shield" size={16} />
          <span>切换后清空当前选择与旧预览；原配置库及其历史备份会完整保留。</span>
        </div>
      </div>
      <footer className="dialog-footer">
        <button type="button" className="button" disabled={busy} onClick={onClose}>
          取消
        </button>
        <button
          type="button"
          className="button primary"
          data-action="confirm-switch-library"
          disabled={busy || !view || !path.trim()}
          onClick={() => void submit()}
        >
          {busy ? '正在切换…' : '使用此配置库'}
        </button>
      </footer>
    </Dialog>
  );
}
