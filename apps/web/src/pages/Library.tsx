import { useMemo, useState } from 'react';
import type { HistoryItem, LibraryRecord, LibraryView, PackageDocument, Profile, TargetStatus } from '../types';
import type { SessionState } from '../session';
import { cellKey } from '../selection';
import { AgentBadge } from '../components/AgentBadge';
import { Icon } from '../components/Icon';
import { Status } from '../components/Status';
import { message } from '../messages';

export function Library({
  library,
  error,
  statusError,
  profiles,
  statuses,
  selectedId,
  onSelect,
  document,
  documentError,
  state,
  onTarget,
  onImport,
  onAgents,
  onEdit,
  onSwitchLibrary,
  onOpenMatrix,
  onRefresh,
  history,
}: {
  library: LibraryView | null;
  error: string | null;
  statusError: string | null;
  profiles: Profile[];
  statuses: TargetStatus[];
  selectedId: string;
  onSelect: (id: string) => void;
  document: PackageDocument | null;
  documentError: string | null;
  state: SessionState;
  onTarget: (key: string, value: boolean) => void;
  onImport: () => void;
  onAgents: () => void;
  onEdit: () => void;
  onSwitchLibrary: () => void;
  onOpenMatrix: () => void;
  onRefresh: () => void;
  history: HistoryItem[];
}) {
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<'all' | 'different' | 'missing'>('all');
  const busy = state.phase === 'confirming';

  const cells = (id: string) => statuses.filter(s => s.packageId === id);

  function overall(record: LibraryRecord) {
    if (statusError) return 'unknown';
    const c = cells(record.id).filter(item => item.state !== 'unsupported');
    return c.some(item => item.state === 'different')
      ? 'different'
      : c.some(item => item.state === 'equal')
        ? 'equal'
        : c.length && c.every(item => item.state === 'missing')
          ? 'missing'
          : 'unknown';
  }

  const records = library?.records || [];
  const selected = records.find(r => r.id === selectedId);
  const counts = {
    all: records.length,
    different: records.filter(r => overall(r) === 'different').length,
    missing: records.filter(r => overall(r) === 'missing').length,
  };

  const shown = useMemo(
    () =>
      records.filter(
        r =>
          (filter === 'all' || overall(r) === filter) &&
          `${r.name} ${r.displayName} ${r.description}`.toLowerCase().includes(search.toLowerCase()),
      ),
    [records, filter, search, statuses, statusError],
  );

  const old = history
    .flatMap(h => h.records)
    .filter(r => r.packageId === selectedId && r.nameAtWrite !== selected?.name);

  function sourceLabel(record: LibraryRecord) {
    if (record.sourceAgent) {
      return profiles.find(p => p.id === record.sourceAgent)?.displayName || record.sourceAgent;
    }
    return record.sourceKind === 'demo' ? '演示资料' : '本地目录';
  }

  return (
    <section className="library-page" aria-label="Skill 配置库">
      <header className="page-heading">
        <div>
          <h1>我的 Skills</h1>
          <p className="sub">维护一份完整 Skill 包，预览逐文件影响后按需应用到你的 Agent。</p>
        </div>
        <div className="heading-actions">
          <button type="button" className="button" onClick={onAgents} disabled={busy}>
            <Icon name="import" size={16} />
            从 Agent 导入
          </button>
          <button
            type="button"
            className="button primary"
            data-action="import-local"
            onClick={onImport}
            disabled={busy || !!error}
          >
            <Icon name="plus" size={16} />
            导入到库
          </button>
        </div>
      </header>

      {error ? (
        <div className="welcome-state error-state" role="alert">
          <div className="welcome-symbol">
            <Icon name="warning" size={28} />
          </div>
          <h2>无法读取配置库</h2>
          <p>{message(error)}</p>
          <code className="path">{state.libraryPath}</code>
          <div className="heading-actions" style={{ marginTop: 18 }}>
            <button className="button primary" type="button" onClick={onSwitchLibrary} disabled={busy}>
              <Icon name="folder" size={16} />
              检查配置库位置
            </button>
            <button className="button" type="button" onClick={onRefresh} disabled={busy}>
              <Icon name="refresh" size={16} />
              重新读取
            </button>
          </div>
          <small>原文件保持不变。你仍可在 Agents 或变更历史页面核对状态与恢复备份。</small>
        </div>
      ) : !library ? (
        <div className="welcome-state loading" role="status">
          <span className="loading-orbit" />
          <h2>正在读取配置库…</h2>
          <p>只读检查完整包与各 Agent 目录状态，不会写入任何文件。</p>
        </div>
      ) : !records.length ? (
        <div className="welcome-state">
          <div className="welcome-symbol">
            <Icon name="library" size={30} />
          </div>
          <h2>还没有 Skill</h2>
          <p>已经配好 Agent？可直接从 Agent 导入，也可从本地目录导入一个完整 Skill 包。</p>
          <div className="heading-actions">
            <button className="button" type="button" onClick={onAgents} disabled={busy}>
              <Icon name="import" size={16} />
              从 Agent 导入
            </button>
            <button
              className="button primary"
              type="button"
              data-action="empty-import"
              onClick={onImport}
              disabled={busy}
            >
              <Icon name="plus" size={16} />
              从本地目录导入
            </button>
          </div>
          <button type="button" className="text-button" onClick={onSwitchLibrary} disabled={busy}>
            切换已有配置库目录
          </button>
          <small>导入只保存到配置库，不会自动分发或修改任何 Agent。</small>
        </div>
      ) : (
        <>
          <div className="library-tools">
            <label className="search-field">
              <Icon name="search" size={16} />
              <input
                aria-label="搜索 Skill"
                placeholder="搜索名称或说明"
                value={search}
                onChange={e => setSearch(e.target.value)}
              />
            </label>

            <div className="filter-pills" role="group" aria-label="库筛选">
              {(['all', 'different', 'missing'] as const).map((f, i) => (
                <button
                  type="button"
                  className={`filter-pill${filter === f ? ' active' : ''}`}
                  key={f}
                  aria-pressed={filter === f}
                  onClick={() => setFilter(f)}
                >
                  {['全部', '待更新', '未分发'][i]}
                  <small>{counts[f]}</small>
                </button>
              ))}
            </div>

            <div className="spacer" />

            <button
              type="button"
              className="button quiet small"
              data-action="open-matrix"
              disabled={busy || !profiles.some(p => p.id)}
              onClick={onOpenMatrix}
            >
              <Icon name="grid" size={16} />
              批量矩阵
            </button>
          </div>

          {statusError && (
            <div className="notice warning error-block">
              <Icon name="warning" size={16} />
              <span>无法读取目标状态。{message(statusError)} 库内容仍可查看，应用前会重新校验。</span>
            </div>
          )}

          <div className={`library-workspace${selected ? ' with-inspector' : ''}`}>
            <div className="library-table-wrap" aria-label="Skill 列表">
              <table className="library-table">
                <thead>
                  <tr>
                    <th>Skill 完整包</th>
                    <th>来源</th>
                    <th>当前分发状态</th>
                    <th>
                      <span className="sr-only">操作</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {shown.map(r => {
                    const rowState = overall(r);
                    const present = statusError
                      ? []
                      : cells(r.id).filter(c => c.state === 'equal' || c.state === 'different');
                    const isQueued = state.selection.some(s => s.startsWith(`["${r.id}",`));
                    const focused = selectedId === r.id;
                    return (
                      <tr key={r.id} className={focused ? 'focused' : ''}>
                        <td>
                          <button
                            type="button"
                            className="entry-name"
                            data-skill={r.id}
                            aria-pressed={focused}
                            onClick={() => onSelect(r.id)}
                          >
                            <span className="entry-symbol">
                              <Icon name="skills" size={20} />
                            </span>
                            <span>
                              <b>{r.displayName || r.name}</b>
                              <small>{r.description || '完整 Skill 目录包'}</small>
                            </span>
                          </button>
                        </td>
                        <td>
                          <span className="source-chip">
                            {r.sourceAgent ? (
                              <AgentBadge agentId={r.sourceAgent} size="sm" />
                            ) : (
                              <Icon name="folder" size={14} />
                            )}
                            <span>{sourceLabel(r)}</span>
                          </span>
                        </td>
                        <td>
                          <div className="row-status">
                            <span className="row-status-line">
                              {present.length > 0 && (
                                <span className="agent-stack">
                                  {present.slice(0, 5).map(c => (
                                    <AgentBadge key={c.agentId} agentId={c.agentId} size="sm" />
                                  ))}
                                </span>
                              )}
                              <Status state={rowState} />
                            </span>
                            {isQueued && <small className="selected-note">已加入本次变更</small>}
                          </div>
                        </td>
                        <td>
                          <button
                            type="button"
                            className="button quiet row-action"
                            aria-label={`选择目标 ${r.displayName || r.name}`}
                            onClick={() => onSelect(r.id)}
                          >
                            选择目标
                            <Icon name="chevron" size={15} />
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {!shown.length && (
                <div className="empty-search">
                  <h3>没有匹配的 Skill</h3>
                  <p className="muted">换个关键词，或切回“全部”筛选。</p>
                  <button
                    type="button"
                    className="text-button"
                    onClick={() => {
                      setSearch('');
                      setFilter('all');
                    }}
                  >
                    清除筛选
                  </button>
                </div>
              )}
            </div>

            {selected && (
              <aside className="entry-inspector inspector" aria-label="Skill 详情">
                <div className="inspector-title">
                  <span className="inspector-kicker">完整包详情</span>
                  <button
                    type="button"
                    className="button quiet icon-button small"
                    aria-label="收起详情"
                    onClick={() => onSelect('')}
                  >
                    <Icon name="close" size={15} />
                  </button>
                </div>

                <h2>{selected.name}</h2>
                <p className="muted inspector-description">{selected.description}</p>

                <div className="inspector-meta">
                  <span>完整包 · {document?.files.length ?? '…'} 个文件</span>
                  <span>来源：{sourceLabel(selected)}</span>
                </div>

                <button
                  type="button"
                  className="button edit-entry"
                  data-action="edit-skill"
                  onClick={onEdit}
                  disabled={busy || document?.document === null}
                >
                  <Icon name="edit" size={16} />
                  编辑 SKILL.md
                </button>

                <section className="inspector-section">
                  <h3>这次要应用到</h3>
                  <p className="field-hint">
                    勾选只记录本次配对。共享目录的额外读取影响将在预览中完整列出。
                  </p>
                  {profiles
                    .filter((p): p is Profile & { id: string } => !!p.id)
                    .map(p => {
                      const s = cells(selected.id).find(c => c.agentId === p.id);
                      const key = cellKey(selected.id, p.id);
                      const checked = state.selection.includes(key);
                      const disabled =
                        busy ||
                        !['builtin', 'enabled'].includes(p.state) ||
                        ['unsupported', 'blocked', 'read_error'].includes(s?.state || '');
                      const st = statusError
                        ? 'read_error'
                        : s?.state || (['builtin', 'enabled'].includes(p.state) ? 'unknown' : p.state);
                      return (
                        <label
                          className={`agent-target target${checked ? ' chosen' : ''}${disabled ? ' unavailable' : ''}`}
                          key={p.id}
                        >
                          <input
                            type="checkbox"
                            data-target={p.id}
                            checked={checked}
                            disabled={disabled}
                            onChange={e => onTarget(key, e.target.checked)}
                          />
                          <AgentBadge agentId={p.id} size="md" />
                          <span className="agent-target-copy target-copy">
                            <strong>{p.displayName}</strong>
                            <Status state={st} />
                          </span>
                          {!statusError && s?.state === 'equal' && <Icon name="check" size={15} />}
                        </label>
                      );
                    })}
                </section>

                <details className="technical-details" open>
                  <summary>包内文件与来源路径（{document?.files.length ?? '…'}）</summary>
                  <div className="source-path" style={{ marginTop: 10 }}>
                    <span>导入来源</span>
                    <code className="path">{selected.sourcePath}</code>
                  </div>
                  {documentError ? (
                    <p className="muted">{message(documentError)}</p>
                  ) : (
                    <ul className="file-list">
                      {document?.files.map(f => (
                        <li key={f}>{f}</li>
                      ))}
                    </ul>
                  )}
                  {document?.issues.map(i => (
                    <p className="form-hint" key={i}>
                      {message(i)}
                    </p>
                  ))}
                </details>

                {old.length > 0 && (
                  <section className="inspector-section" style={{ marginTop: 14 }}>
                    <div className="notice warning">历史名称位置可能仍被 Agent 读取，请逐项核对。</div>
                    {old.map((r, i) => (
                      <p className="path section-space" key={i}>
                        {r.nameAtWrite}
                        <br />
                        {r.physicalPath}
                      </p>
                    ))}
                  </section>
                )}
              </aside>
            )}
          </div>

          <p className="library-footnote">
            <Icon name="shield" size={15} />
            <span>这里显示磁盘目录与完整字节对比状态，Agent 真实加载状态需在目标客户端核对。</span>
          </p>
        </>
      )}
    </section>
  );
}
