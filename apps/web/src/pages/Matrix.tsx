import type { LibraryRecord, Profile, TargetStatus } from '../types';
import type { SessionState } from '../session';
import { cellKey, selectionFromCells } from '../selection';
import { AgentBadge } from '../components/AgentBadge';
import { Dialog } from '../components/Dialog';
import { Icon } from '../components/Icon';
import { label } from '../messages';

export function MatrixDialog({
  records,
  profiles,
  statuses,
  statusError,
  state,
  onTarget,
  onClear,
  onClose,
  onPreview,
}: {
  records: LibraryRecord[];
  profiles: Profile[];
  statuses: TargetStatus[];
  statusError: string | null;
  state: SessionState;
  onTarget: (key: string, checked: boolean) => void;
  onClear: () => void;
  onClose: () => void;
  onPreview: () => void;
}) {
  const busy = state.phase === 'confirming';
  const validProfiles = profiles.filter((p): p is Profile & { id: string } => !!p.id);
  const pairs = selectionFromCells(state.selection);
  const skillCount = new Set(pairs.map(p => p.packageId)).size;
  const agentCount = new Set(pairs.map(p => p.agent)).size;

  function cellState(packageId: string, profile: Profile & { id: string }) {
    if (!['builtin', 'enabled'].includes(profile.state)) return profile.state;
    if (statusError) return 'read_error';
    return statuses.find(s => s.packageId === packageId && s.agentId === profile.id)?.state || 'unknown';
  }

  function cellDisabled(packageId: string, profile: Profile & { id: string }) {
    if (busy || !['builtin', 'enabled'].includes(profile.state)) return true;
    const st = cellState(packageId, profile);
    return ['unsupported', 'blocked', 'read_error'].includes(st);
  }

  function toggleColumn(profile: Profile & { id: string }) {
    if (busy) return;
    const keys = records
      .filter(r => !cellDisabled(r.id, profile))
      .map(r => cellKey(r.id, profile.id));
    if (!keys.length) return;
    const allSelected = keys.every(k => state.selection.includes(k));
    for (const k of keys) {
      onTarget(k, !allSelected);
    }
  }

  return (
    <Dialog title="批量选择矩阵" wide locked={busy} onClose={onClose}>
      <div className="dialog-body">
        <p className="muted">
          高级批量配对视图。每个单元格表示一组 Skill 与目标 Agent，点击列头可整列全选；未勾选不代表卸载或删除。
        </p>

        {!records.length || !validProfiles.length ? (
          <div className="welcome-state compact">
            <h3>当前没有可批量配对的条目</h3>
            <p>请先导入 Skill 包并确保至少有一个已启用的 Agent 档案。</p>
          </div>
        ) : (
          <>
            <div className="matrix-wrap">
              <table className="matrix">
                <thead>
                  <tr>
                    <th className="rowhead">Skill 包</th>
                    {validProfiles.map(p => (
                      <th key={p.id}>
                        <button
                          type="button"
                          className="matrix-col-btn"
                          data-matrix-col={p.id}
                          title={`全选 / 取消全选 ${p.displayName}`}
                          disabled={busy || !['builtin', 'enabled'].includes(p.state)}
                          onClick={() => toggleColumn(p)}
                        >
                          <AgentBadge agentId={p.id} size="sm" />
                          <span>{p.displayName}</span>
                        </button>
                        <div className="matrix-col-meta">{p.source === 'builtin' ? '内置' : '自定义'}</div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {records.map(r => (
                    <tr key={r.id}>
                      <th className="rowhead">
                        <div className="name">{r.displayName || r.name}</div>
                        <div className="meta">{r.name}</div>
                      </th>
                      {validProfiles.map(p => {
                        const key = cellKey(r.id, p.id);
                        const selected = state.selection.includes(key);
                        const st = cellState(r.id, p);
                        const disabled = cellDisabled(r.id, p);
                        const dotTone =
                          st === 'equal'
                            ? 'ok'
                            : st === 'different'
                              ? 'diff'
                              : disabled
                                ? 'unsupported'
                                : 'absent';
                        return (
                          <td key={p.id}>
                            <button
                              type="button"
                              data-matrix-cell={`${r.id}:${p.id}`}
                              className={`cell-btn${selected ? ' sel' : ''}${disabled ? ' unsupported' : ''}`}
                              disabled={disabled}
                              title={`${r.displayName || r.name} → ${p.displayName}（${label(st)}）`}
                              aria-pressed={selected}
                              onClick={() => onTarget(key, !selected)}
                            >
                              <span className={`dot ${dotTone}`} />
                              <span className="tick">{selected ? '✓' : ''}</span>
                            </button>
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div className="legend">
              <span>
                <i className="dot ok" />
                内容一致
              </span>
              <span>
                <i className="dot diff" />
                存在差异
              </span>
              <span>
                <i className="dot absent" />
                未分发
              </span>
              <span>
                <i className="dot unsupported" />
                不可用 / 未启用
              </span>
              <span className="muted">点列头可整列全选；确认前会展示完整字节对比预览</span>
            </div>
          </>
        )}
      </div>

      <footer className="dialog-footer">
        <span className="footer-copy">
          已选 {skillCount} 个 Skill · {agentCount} 个 Agent · {state.selection.length} 组配对
        </span>
        {state.selection.length > 0 && (
          <button type="button" className="button quiet" disabled={busy} onClick={onClear}>
            清空
          </button>
        )}
        <button type="button" className="button" disabled={busy} onClick={onClose}>
          完成选择
        </button>
        <button
          type="button"
          className="button primary"
          data-action="matrix-preview"
          disabled={!state.selection.length || busy}
          onClick={() => {
            onClose();
            onPreview();
          }}
        >
          生成变更预览
          <Icon name="arrow" size={16} />
        </button>
      </footer>
    </Dialog>
  );
}
