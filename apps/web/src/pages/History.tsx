import type { HistoryItem, Recovery } from '../types';
import { AgentBadge } from '../components/AgentBadge';
import { Icon } from '../components/Icon';
import { Status } from '../components/Status';
import { label, message } from '../messages';

export function History({
  items,
  recoveries,
  error,
  recoveryError,
  onRestore,
  onRefresh,
  busy,
}: {
  items: HistoryItem[];
  recoveries: Recovery[];
  error: string | null;
  recoveryError: string | null;
  onRestore: (id: string) => void;
  onRefresh: () => void;
  busy: boolean;
}) {
  return (
    <section className="history-page" aria-busy={busy}>
      <header className="page-heading">
        <div>
          <h1>变更历史</h1>
          <p className="sub">核对逐组写入回执与中心备份；恢复同样通过独立预览与手动确认执行。</p>
        </div>
        <div className="heading-actions">
          <span className="quiet-caption">
            <Icon name="history" size={15} />
            写入前保留完整备份
          </span>
          <button className="button" type="button" onClick={onRefresh} disabled={busy}>
            <Icon name="refresh" size={16} />
            重新读取
          </button>
        </div>
      </header>

      {recoveryError && (
        <div className="notice warning error-block">待恢复任务读取失败：{message(recoveryError)}</div>
      )}

      {recoveries.length > 0 && (
        <section className="panel recovery-panel error-block" aria-label="待恢复任务">
          <div className="panel-head">
            <Icon name="warning" size={17} />
            <span>需要处理的事务</span>
            <div className="spacer" />
            <span className="pill warn">{recoveries.length} 项</span>
          </div>
          <div className="panel-body stack">
            {recoveries.map(r => (
              <article className="backup-card recovery-card" key={r.operationId}>
                <div className="backup-card-top">
                  <span className="backup-icon warn">
                    <Icon name="warning" size={19} />
                  </span>
                  <div className="grow">
                    <h3>
                      {label(r.kind)} · {r.state}
                    </h3>
                    <small className="mono">{r.operationId}</small>
                  </div>
                  <button
                    className="button primary"
                    type="button"
                    data-action="preview-recovery"
                    disabled={busy}
                    onClick={() => onRestore(r.operationId)}
                  >
                    <Icon name="history" size={16} />
                    预览恢复
                  </button>
                </div>
                {r.reason && <p className="form-hint section-space">{message(r.reason)}</p>}
                {r.paths.map(p => (
                  <code className="actual-file-path" key={p}>
                    {p}
                  </code>
                ))}
                <p className="form-hint">保留锁、staging 和归档现场，请通过显式预览恢复处理。</p>
              </article>
            ))}
          </div>
        </section>
      )}

      {error ? (
        <div className="notice error" role="alert">
          无法读取变更历史。{message(error)}
        </div>
      ) : !items.length ? (
        <div className="welcome-state compact">
          <div className="welcome-symbol">
            <Icon name="history" size={28} />
          </div>
          <h2>还没有变更记录</h2>
          <p>导入、保存、应用和恢复的记录会显示在这里。只读预览不会创建备份。</p>
        </div>
      ) : (
        <div className="backup-list" aria-label="变更记录">
          {items.map(h => (
            <article className="backup-card" key={h.operationId}>
              <div className="backup-card-top">
                <span className="backup-icon">
                  <Icon name="folder" size={19} />
                </span>
                <div className="grow">
                  <h3>{h.kind.startsWith('restore:') ? '恢复' : label(h.kind)}</h3>
                  <small>
                    {new Date(h.createdAt * 1000).toLocaleString('zh-CN')} · {h.outcomes.length} 组结果
                  </small>
                </div>
                <button
                  type="button"
                  className="button"
                  data-action="preview-restore"
                  disabled={busy}
                  onClick={() => onRestore(h.operationId)}
                >
                  <Icon name="history" size={16} />
                  预览恢复
                </button>
              </div>

              {h.backupLocation && <code className="actual-file-path">{h.backupLocation}</code>}

              <details>
                <summary>查看备份与逐组结果</summary>
                <p className="path section-space">操作：{h.operationId}</p>
                {h.outcomes.map((o, i) => (
                  <div className="receipt-row" key={i}>
                    <code className="path">{o.path}</code>
                    <span>
                      <Status state={o.outcome} />
                      {o.reason && <p className="form-hint">{message(o.reason)}</p>}
                    </span>
                  </div>
                ))}
                {h.records.map((r, i) => (
                  <div className="history-record" key={i}>
                    <div className="row">
                      <b>{r.nameAtWrite}</b>
                      <span className="pill">{r.scope === 'project' ? '项目范围' : '用户范围'}</span>
                    </div>
                    <code className="path">{r.physicalPath}</code>
                    {r.selectedProfiles.length > 0 && (
                      <div className="row" style={{ marginTop: 6, gap: 6 }}>
                        {r.selectedProfiles.map((p, idx) => (
                          <span className="source-chip" key={`${p.agentId}-${idx}`}>
                            <AgentBadge agentId={p.agentId} size="sm" />
                            <span>{p.agentId}</span>
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </details>

              {h.warnings.map((w, i) => (
                <p className="form-hint section-space" key={i}>
                  {w}
                </p>
              ))}
            </article>
          ))}
        </div>
      )}

      <p className="history-note">
        恢复前会生成独立差异预览并校验目标身份；原先不存在的目录将归档移出加载路径，不执行直接删除。
      </p>
    </section>
  );
}
