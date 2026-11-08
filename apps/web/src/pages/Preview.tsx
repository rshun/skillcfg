import type { FileDiff, Profile } from '../types';
import type { SessionController, SessionState } from '../session';
import { AgentBadge } from '../components/AgentBadge';
import { Dialog } from '../components/Dialog';
import { Icon } from '../components/Icon';
import { Status } from '../components/Status';
import { message } from '../messages';

const kindLabel = (kind: string | null) =>
  kind === 'file' ? '文件' : kind === 'directory' ? '目录' : '不存在';

const changeLabel = (f: FileDiff) =>
  ({
    added: '新增',
    modified: '修改内容',
    removed: '移出加载目录并归档',
    permissions: '仅改变执行权限',
    typeChanged: '文件与目录类型变化',
  })[f.change];

export function Preview({
  state,
  session,
  profiles,
  decisions,
  onDecision,
  onRepreview,
}: {
  state: SessionState;
  session: SessionController;
  profiles: Profile[];
  decisions: Record<string, string>;
  onDecision: (key: string, value: string) => void;
  onRepreview: () => void;
}) {
  const busy = state.phase === 'confirming';
  const ready = state.phase === 'ready';
  const result = state.phase === 'result';
  const title = result
    ? '操作回执'
    : state.kind === 'import'
      ? '预览导入到库'
      : state.kind === 'restore'
        ? '预览恢复'
        : '预览变更';

  const agentName = (id: string) => profiles.find(p => p.id === id)?.displayName || id;
  const wrote = state.receipt?.outcomes.some(o => o.outcome === 'written');
  const issues = state.receipt?.outcomes.some(o =>
    ['blocked', 'failed_restored', 'recovery_required', 'audit_failed'].includes(o.outcome),
  );

  const groups = state.preview?.groups ?? [];
  const writableCount = groups.filter(g => ['new', 'update'].includes(g.disposition)).length;
  const blockedCount = groups.filter(g => g.disposition === 'blocked').length;

  return (
    <Dialog title={title} wide locked={busy} onClose={() => void session.cancel()}>
      <div className="dialog-body" aria-live="polite">
        {state.error && (
          <div className="notice error" role="alert">
            {message(state.error)}
            {result && ' 本次确认已消费，需重新预览。'}
          </div>
        )}

        {state.phase === 'planning' && (
          <div className="read-placeholder loading" role="status">
            <span className="loading-orbit" />
            <p>正在读取当前内容并生成预览…</p>
          </div>
        )}

        {state.phase === 'expired' && (
          <div className="notice warning">预览已过期，请重新预览。</div>
        )}

        {busy && (
          <div className="notice">
            <Icon name="shield" size={16} />
            <span>正在备份、应用并复核文件。请等待逐组回执。</span>
          </div>
        )}

        {result && state.receipt && (
          <section className={`receipt-panel${issues ? ' has-issues' : ''}`}>
            <div className="receipt-summary">
              <span className="receipt-icon">
                <Icon name={issues ? 'warning' : 'check'} size={22} />
              </span>
              <div>
                <p className="result-title">
                  {issues
                    ? '操作包含需要处理的结果'
                    : !wrote
                      ? '本次没有写入内容'
                      : state.kind === 'import'
                        ? '已保存到库，尚未分发'
                        : state.kind === 'restore'
                          ? '恢复操作已完成'
                          : '文件已应用，请在目标 Agent 核对加载。'}
                </p>
                <p className="muted">
                  共 {state.receipt.outcomes.length} 组目标结果 · 连接与运行时加载状态未自动验证
                </p>
              </div>
            </div>

            {issues && (
              <div className="notice warning" style={{ marginBottom: 12 }}>
                请逐组检查结果。已写入的目录与失败项分别保留；审计失败不会抹掉真实写入结果。
              </div>
            )}

            {state.receipt.backupLocation && (
              <code className="actual-file-path">备份：{state.receipt.backupLocation}</code>
            )}

            <div className="outcome-list">
              {state.receipt.outcomes.map((o, i) => (
                <div className="receipt-row" key={i}>
                  <code className="path">{o.path}</code>
                  <span>
                    <Status state={o.outcome} />
                    {o.reason && <p className="form-hint">{message(o.reason)}</p>}
                  </span>
                </div>
              ))}
            </div>

            {state.receipt.operationId && (
              <p className="form-hint" style={{ marginTop: 10 }}>
                操作：{state.receipt.operationId}
              </p>
            )}
          </section>
        )}

        {state.preview && !result && (
          <>
            <div className="preview-intro">
              <span className="preview-symbol">
                <Icon name="shield" size={22} />
              </span>
              <div>
                <h3>先核对实际目录与影响，再确认写入</h3>
                <p>确认表示同意以下实际目录与列出的影响。写入前会完整备份旧包，目录切换时可能短暂不可用。</p>
              </div>
            </div>

            <div className="preview-summary">
              <div className="summary-stat">
                <b>{groups.length}</b>
                <span>涉及物理目录</span>
              </div>
              <div className="summary-stat">
                <b>{writableCount}</b>
                <span>待新建 / 替换</span>
              </div>
              {blockedCount > 0 && (
                <div className="summary-stat warn">
                  <b>{blockedCount}</b>
                  <span>已阻止目标</span>
                </div>
              )}
            </div>

            <code className="actual-file-path">备份位置：{state.preview.backupLocation}</code>

            {state.preview.warnings.map((w, i) => (
              <div className="notice warning" key={i}>
                {w}
              </div>
            ))}
            {state.preview.blocked && (
              <div className="notice error">预览包含阻止项，请处理后重新预览。</div>
            )}
            {!groups.length && <p className="muted">本次没有目录变化。</p>}

            {groups.map(g => (
              <section className="preview-group file-plan" key={g.targetKey}>
                <header className="group-head">
                  <div className="grow">
                    <div className="row" style={{ gap: 8 }}>
                      {g.selectedAgents.map(a => (
                        <AgentBadge key={a} agentId={a} size="sm" />
                      ))}
                      <Status state={g.disposition} />
                    </div>
                    <code className="actual-file-path" style={{ marginBottom: 0 }}>
                      {g.path}
                    </code>
                  </div>
                  {state.kind === 'distribution' && ['update', 'skipped'].includes(g.disposition) && (
                    <label className="field" style={{ minWidth: 170 }}>
                      <span className="form-hint">同名目录处理</span>
                      <select
                        aria-label={`冲突策略 ${g.path}`}
                        disabled={!ready}
                        value={decisions[g.targetKey] || 'skip'}
                        onChange={e => onDecision(g.targetKey, e.target.value)}
                      >
                        <option value="skip">跳过（默认）</option>
                        <option value="replace">替换并归档旧包</option>
                      </select>
                    </label>
                  )}
                </header>

                <div className="group-body">
                  <div className="reader-list">
                    <p>主动选择：{g.selectedAgents.map(agentName).join('、') || '库 / 本地资料'}</p>
                    {g.possibleReaders.filter(r => !g.selectedAgents.includes(r.agent)).length > 0 && (
                      <p className="warning-text">
                        可能额外读取：
                        {g.possibleReaders
                          .filter(r => !g.selectedAgents.includes(r.agent))
                          .map(
                            r =>
                              `${agentName(r.agent)}（${
                                r.evidence === 'unknown'
                                  ? '读取条件未知'
                                  : r.evidence === 'configDisabled'
                                    ? '配置声明关闭'
                                    : '声明可读取'
                              }）`,
                          )
                          .join('、')}
                      </p>
                    )}
                  </div>

                  {g.reason && <p className="form-hint section-space">{message(g.reason)}</p>}

                  {g.files.map(f => (
                    <details className="diff-file" key={f.path}>
                      <summary>
                        {f.path} · {changeLabel(f)}
                        {f.beforeKind === 'directory' || f.afterKind === 'directory' ? '（目录）' : ''}{' '}
                        <span className="sub">
                          {f.binary ? '二进制' : `${f.beforeBytes} → ${f.afterBytes} 字节`}
                        </span>
                      </summary>
                      {f.change === 'removed' && (
                        <p className="notice warning" style={{ marginTop: 8 }}>
                          该文件将随旧包归档，并移出加载目录。
                        </p>
                      )}
                      {f.change === 'permissions' && (
                        <p className="notice" style={{ marginTop: 8 }}>
                          执行权限：{f.beforeExecutable ? '可执行' : '不可执行'} →{' '}
                          {f.afterExecutable ? '可执行' : '不可执行'}
                        </p>
                      )}
                      {f.beforeKind === 'directory' || f.afterKind === 'directory' ? (
                        <p className="muted" style={{ marginTop: 8 }}>
                          变更前：{kindLabel(f.beforeKind)}；变更后：{kindLabel(f.afterKind)}。
                        </p>
                      ) : f.binary ? (
                        <p className="muted" style={{ marginTop: 8 }}>
                          仅展示二进制摘要：原文件 {f.beforeBytes} 字节，变更后 {f.afterBytes} 字节。
                        </p>
                      ) : (
                        <div className="diff-sides">
                          <section className="diff-side">
                            <span>当前内容</span>
                            <pre>{f.before ?? '（原先不存在）'}</pre>
                          </section>
                          <section className="diff-side">
                            <span>确认后内容</span>
                            <pre>{f.after ?? '（移入归档）'}</pre>
                          </section>
                        </div>
                      )}
                      {f.truncated && (
                        <p className="form-hint">展示内容已截断；实际操作使用后端完整字节。</p>
                      )}
                    </details>
                  ))}
                </div>
              </section>
            ))}
          </>
        )}
      </div>

      <footer className="dialog-footer">
        <span className="footer-copy">
          {busy
            ? '正在执行，暂不能取消'
            : result
              ? '逐组结果以回执为准'
              : '预览有效期 10 分钟；任何确认尝试都会消费本次预览。'}
        </span>
        <button type="button" className="button" disabled={busy} onClick={() => void session.cancel()}>
          {result ? '完成' : '取消'}
        </button>
        {state.phase === 'expired' && (
          <button type="button" className="button primary" onClick={onRepreview}>
            重新预览
          </button>
        )}
        {ready && !state.preview?.blocked && (
          <button
            type="button"
            className="button primary"
            data-action="confirm-preview"
            onClick={() => void session.confirm()}
          >
            {state.kind === 'import'
              ? '确认保存到库'
              : state.kind === 'restore'
                ? '确认恢复'
                : '确认应用'}
          </button>
        )}
      </footer>
    </Dialog>
  );
}
