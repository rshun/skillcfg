import { useEffect, useState } from 'react';
import type { Detection, Inventory, Profile, Receipt, SettingsView } from '../types';
import type { SessionController } from '../session';
import { errorCode } from '../session';
import { AgentBadge } from '../components/AgentBadge';
import { Icon } from '../components/Icon';
import { Status } from '../components/Status';
import { MutationReceipt, incompleteReceipt } from '../components/MutationReceipt';
import { Dialog } from '../components/Dialog';
import { message } from '../messages';

export function Agents({
  profiles,
  detections,
  detectionError,
  error,
  session,
  onProfile,
  onImport,
  onChanged,
  busy,
}: {
  profiles: Profile[];
  detections: Detection[];
  detectionError: string | null;
  error: string | null;
  session: SessionController;
  onProfile: (profile: Profile | null) => void;
  onImport: (source: string) => void;
  onChanged: () => Promise<void>;
  busy: boolean;
}) {
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [disable, setDisable] = useState<Profile | null>(null);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [variant, setVariant] = useState('');

  const profile = profiles.find(p => p.id === selected);

  useEffect(() => {
    let active = true;
    setInventory(null);
    setReadError(null);
    setVariant(profile?.currentVariant || '');
    if (profile?.id && ['builtin', 'enabled'].includes(profile.state)) {
      setReading(true);
      void session
        .read<Inventory>('list_agent_skills', { agentId: profile.id })
        .then(v => {
          if (active && v) setInventory(v);
        })
        .catch(e => {
          if (active) setReadError(errorCode(e));
        })
        .finally(() => {
          if (active) setReading(false);
        });
    } else {
      setReading(false);
    }
    return () => {
      active = false;
    };
  }, [selected, profile?.state, profile?.revision, profile?.currentVariant, session.state.generation, session]);

  async function saveVariant() {
    if (!profile?.id) return;
    setSaving(true);
    setActionError(null);
    try {
      const view = await session.call<SettingsView>('load_settings');
      await session.call('save_settings', {
        expectedRevision: view.revision,
        settings: {
          ...view.settings,
          agentVariants: { ...view.settings.agentVariants, [profile.id]: variant },
        },
      });
      await onChanged();
    } catch (e) {
      setActionError(errorCode(e));
    } finally {
      setSaving(false);
    }
  }

  async function disableProfile() {
    if (!disable?.id || !disable.revision) return;
    setSaving(true);
    setActionError(null);
    setReceipt(null);
    try {
      const fresh = (await session.call<Profile[]>('list_agent_profiles')).find(p => p.id === disable.id);
      if (!fresh?.revision || JSON.stringify({ ...fresh, revision: null }) !== JSON.stringify({ ...disable, revision: null })) {
        throw { code: 'profile_changed' };
      }
      const result = await session.call<Receipt>('set_agent_profile_enabled', {
        agentId: disable.id,
        enabled: false,
        expectedRevision: fresh.revision,
      });
      if (incompleteReceipt(result)) {
        setReceipt(result);
        return;
      }
      setDisable(null);
      await onChanged();
    } catch (e) {
      setActionError(errorCode(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="agents-page">
      <header className="page-heading">
        <div>
          <h1>Agents</h1>
          <p className="sub">检查读取目录与安装状态，安全导入已有 Skill，或通过声明式档案接入新 Agent。</p>
        </div>
        <div className="heading-actions">
          <span className="quiet-caption">
            <Icon name="shield" size={15} />
            只读扫描不改原文件
          </span>
          <button
            className="button primary"
            type="button"
            data-action="add-profile"
            disabled={busy}
            onClick={() => onProfile(null)}
          >
            <Icon name="plus" size={16} />
            添加 Agent
          </button>
        </div>
      </header>

      <div className="notice error-block">
        <Icon name="shield" size={16} />
        <span>目录规则来自声明式档案。文件已写入并不等于 Agent 已加载，首次使用请在目标 Agent 核对。</span>
      </div>

      {detectionError && (
        <div className="notice warning error-block">无法检查安装状态：{message(detectionError)}</div>
      )}
      {error && (
        <div className="notice error error-block" role="alert">
          无法读取 Agent 档案。{message(error)}
        </div>
      )}
      {actionError && !disable && (
        <div className="notice error error-block" role="alert">
          {message(actionError)}
        </div>
      )}

      <div className="agents-workspace">
        <aside className="agent-list" aria-label="Agent 档案列表">
          <div className="section-caption">Agent 档案 · {profiles.length}</div>
          {profiles.map((p, i) => {
            const installed = detections.find(d => d.agentId === p.id)?.installed;
            const detail = detectionError
              ? '无法检查安装状态'
              : installed
                ? '已检测到 Agent'
                : p.roots.some(r => r.exists)
                  ? '仅目录存在'
                  : '未发现 Agent';
            const active = selected === p.id && !!p.id;
            return (
              <button
                type="button"
                className={`agent-choice${active ? ' active' : ''}`}
                data-agent={p.id || `invalid-${i}`}
                key={p.id || i}
                disabled={!p.id || busy}
                aria-pressed={active}
                onClick={() => setSelected(p.id)}
              >
                <AgentBadge agentId={p.id} size="lg" />
                <span className="agent-copy">
                  <span className="row spread">
                    <b>{p.displayName}</b>
                    <span className="pill">{p.source === 'builtin' ? '内置' : '用户档案'}</span>
                  </span>
                  <small className="agent-choice-meta">
                    <Status state={p.state} />
                    <span>· {detail}</span>
                  </small>
                  {p.issues.length > 0 && (
                    <small className="agent-issue">{p.issues.map(message).join(' ')}</small>
                  )}
                </span>
                <Icon name="chevron" size={15} />
              </button>
            );
          })}
        </aside>

        <div className="agent-config-area">
          {profile ? (
            <section className="panel" aria-label={`${profile.displayName} 详情`}>
              <div className="panel-head">
                <AgentBadge agentId={profile.id} size="lg" />
                <div>
                  <h2>{profile.displayName}</h2>
                  <code className="path">{profile.id}</code>
                </div>
                <span className="pill">{profile.source === 'builtin' ? '内置档案' : '用户档案'}</span>
                <div className="spacer" />
                <div className="row">
                  <button
                    type="button"
                    className="button"
                    disabled={busy}
                    onClick={() => onProfile(profile)}
                  >
                    {profile.source === 'builtin' ? '查看 / 配置目录' : '编辑档案'}
                  </button>
                  {['builtin', 'enabled'].includes(profile.state) ? (
                    <button
                      type="button"
                      className="button quiet"
                      disabled={busy || !profile.revision}
                      onClick={() => {
                        setActionError(null);
                        setReceipt(null);
                        setDisable(profile);
                      }}
                    >
                      停用档案
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="button primary"
                      disabled={busy}
                      onClick={() => onProfile(profile)}
                    >
                      预览启用
                    </button>
                  )}
                </div>
              </div>

              <div className="panel-body stack">
                {profile.variants.length > 1 && (
                  <div className="variant-box">
                    <label className="field">
                      <span>当前 Variant</span>
                      <select value={variant} disabled={busy || saving} onChange={e => setVariant(e.target.value)}>
                        {profile.variants.map(v => (
                          <option key={v} value={v}>
                            {v}
                          </option>
                        ))}
                      </select>
                    </label>
                    <div className="row spread">
                      <p className="form-hint">更改后清空本次选择和旧预览，已分发文件保留。</p>
                      <button
                        type="button"
                        className="button"
                        disabled={busy || saving || variant === profile.currentVariant}
                        onClick={() => void saveVariant()}
                      >
                        保存 Variant 选择
                      </button>
                    </div>
                  </div>
                )}

                <div>
                  <h3 className="sub-heading">解析目录</h3>
                  {!profile.roots.length && (
                    <p className="muted">当前没有已授权的读取目录，请先预览并确认档案。</p>
                  )}
                  <div className="stack" style={{ gap: 10 }}>
                    {profile.roots.map((r, i) => (
                      <div className="source-path" key={`${r.rootId}-${i}`}>
                        <div className="row spread">
                          <span>{r.rootId}</span>
                          <span className={`pill${r.writable ? ' ok' : ''}`}>
                            {r.writable ? '可作为写入根' : '只读根'}
                          </span>
                        </div>
                        <code className="path">{r.path}</code>
                        {r.rootWasLink && (
                          <code className="path">实际位置：{r.actualPath || '无法解析'}</code>
                        )}
                      </div>
                    ))}
                  </div>
                </div>

                <div>
                  <h3 className="sub-heading">已有 Skill</h3>
                  {reading ? (
                    <div className="read-placeholder loading" role="status">
                      <span className="loading-orbit" />
                      <p>正在只读扫描…</p>
                    </div>
                  ) : readError ? (
                    <p className="notice warning">{message(readError)}</p>
                  ) : inventory ? (
                    <>
                      {inventory.errors.map((e, i) => (
                        <p key={i} className="form-hint">
                          {message(e)}
                        </p>
                      ))}
                      {!inventory.items.length && (
                        <p className="muted section-space">当前读取目录中没有发现可列出的 Skill。</p>
                      )}
                      {inventory.items.map((item, i) => (
                        <article className="inventory-item" key={i}>
                          <div className="row spread">
                            <div className="row">
                              <span className="entry-symbol small">
                                <Icon name="skills" size={16} />
                              </span>
                              <div>
                                <h3>{item.name || '无效 Skill'}</h3>
                                {item.description && <p className="muted">{item.description}</p>}
                              </div>
                            </div>
                          </div>
                          {item.sources.map(s => (
                            <div className="inventory-source" key={s.path}>
                              <div>
                                <code className="path">{s.path}</code>
                                {s.rootWasLink && (
                                  <code className="path" style={{ display: 'block' }}>
                                    实际位置：{s.actualPath || '无法解析'}
                                  </code>
                                )}
                              </div>
                              <button
                                type="button"
                                className="button small"
                                disabled={busy || item.issues.length > 0}
                                onClick={() => onImport(s.path)}
                              >
                                <Icon name="import" size={15} />
                                导入到库
                              </button>
                            </div>
                          ))}
                          {item.issues.map(code => (
                            <p className="form-hint" key={code}>
                              {message(code)}
                            </p>
                          ))}
                        </article>
                      ))}
                    </>
                  ) : (
                    <p className="muted section-space">启用档案并确认读取根后，再扫描该 Agent。</p>
                  )}
                </div>
              </div>
            </section>
          ) : (
            <div className="agent-empty">
              <div className="welcome-symbol">
                <Icon name="agents" size={30} />
              </div>
              <h2>选择一个 Agent，查看它的 Skill 目录</h2>
              <p>检查解析根目录与符号链接别名，也可以把已有完整 Skill 包安全导入到配置库。</p>
            </div>
          )}
        </div>
      </div>

      {disable && (
        <Dialog title="停用 Agent 档案" locked={saving} onClose={() => setDisable(null)}>
          <div className="dialog-body">
            <p>停用 {disable.displayName} 后，它将不再作为本次分发的目标。</p>
            <div className="notice">
              <Icon name="shield" size={16} />
              <span>停用档案不会卸载或删除已分发的 Skill，其他 Agent 仍可能读取共享目录。</span>
            </div>
            {actionError && (
              <p className="notice error" role="alert">
                {message(actionError)}
              </p>
            )}
            <MutationReceipt receipt={receipt} />
          </div>
          <footer className="dialog-footer">
            <button type="button" className="button" disabled={saving} onClick={() => setDisable(null)}>
              取消
            </button>
            <button
              type="button"
              className="button primary"
              data-action="confirm-disable"
              disabled={saving}
              onClick={() => void disableProfile()}
            >
              {saving ? '正在保存…' : '确认停用'}
            </button>
          </footer>
        </Dialog>
      )}
    </section>
  );
}
