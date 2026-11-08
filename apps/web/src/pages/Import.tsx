import { useEffect, useRef, useState } from 'react';
import type { ImportChoice, ImportScan, LibraryRecord, Transport } from '../types';
import { Dialog } from '../components/Dialog';
import { Icon } from '../components/Icon';
import { message } from '../messages';
import { errorCode } from '../session';

export function Import({
  source: initial = '',
  records,
  call,
  onClose,
  onPreview,
}: {
  source?: string;
  records: LibraryRecord[];
  call: Transport;
  onClose: () => void;
  onPreview: (scanId: string, choices: ImportChoice[], source: string) => Promise<void>;
}) {
  const [source, setSource] = useState(initial);
  const [scan, setScan] = useState<ImportScan | null>(null);
  const [choices, setChoices] = useState<Record<string, ImportChoice>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);

  useEffect(() => () => {
    ++epoch.current;
  }, []);

  async function scanSource() {
    const current = ++epoch.current;
    setBusy(true);
    setError(null);
    setScan(null);
    setChoices({});
    try {
      const result = await call<ImportScan>('scan_import', { source });
      if (current === epoch.current) setScan(result);
    } catch (e) {
      if (current === epoch.current) setError(errorCode(e));
    } finally {
      if (current === epoch.current) setBusy(false);
    }
  }

  async function pick() {
    try {
      const selected = await call<string | null>('choose_directory', { initial: source || null });
      if (selected) {
        ++epoch.current;
        setSource(selected);
        setScan(null);
        setChoices({});
      }
    } catch (e) {
      setError(errorCode(e));
    }
  }

  function choose(itemId: string, checked: boolean) {
    const next = { ...choices };
    if (checked) {
      const item = scan!.items.find(i => i.itemId === itemId)!;
      next[itemId] = {
        itemId,
        action: records.some(r => r.name === item.name) ? 'skip' : 'new',
        replaceId: null,
      };
    } else {
      delete next[itemId];
    }
    setChoices(next);
  }

  const actionable = Object.values(choices).filter(c => c.action !== 'skip');

  return (
    <Dialog title="导入到库" wide onClose={onClose}>
      <div className="dialog-body">
        <div className="import-steps">
          <span className={!scan ? 'active' : ''}>1. 选择并扫描目录</span>
          <span>→</span>
          <span className={scan ? 'active' : ''}>2. 选择完整包与同名策略</span>
          <span>→</span>
          <span>3. 预览保存到库</span>
        </div>

        <p className="muted">
          选择包含 SKILL.md 的包目录，或包含多个 Skill 的父目录。扫描不会改动来源文件。
        </p>

        <label className="field">
          <span>来源目录</span>
          <span className="location-input">
            <input
              value={source}
              spellCheck={false}
              disabled={busy}
              placeholder="输入或选择 Skill 所在目录"
              onChange={e => {
                ++epoch.current;
                setSource(e.target.value);
                setScan(null);
                setChoices({});
              }}
            />
            <button
              type="button"
              className="button"
              data-action="pick-import"
              onClick={() => void pick()}
              disabled={busy}
            >
              <Icon name="folder" size={16} />
              选择目录
            </button>
          </span>
        </label>

        <div className="row">
          <button
            type="button"
            className="button"
            data-action="scan-import"
            disabled={!source.trim() || busy}
            onClick={() => void scanSource()}
          >
            <Icon name="search" size={16} />
            {busy ? '正在扫描…' : '扫描完整包'}
          </button>
        </div>

        {error && (
          <div className="notice error" role="alert">
            {message(error)}
          </div>
        )}

        {scan && !scan.items.length && (
          <div className="welcome-state compact">
            <h3>没有找到 Skill 包</h3>
            <p>请确认所选目录下存在 SKILL.md，或换一个来源目录。</p>
          </div>
        )}

        {scan?.items.map(item => {
          const selected = choices[item.itemId];
          const existing = records.filter(r => r.name === item.name);
          return (
            <div className={`scan-item${selected ? ' chosen' : ''}`} key={item.itemId}>
              <input
                type="checkbox"
                data-import-item={item.itemId}
                aria-label={`导入 ${item.name}`}
                checked={!!selected}
                disabled={item.issues.length > 0}
                onChange={e => choose(item.itemId, e.target.checked)}
              />
              <div>
                <div className="row spread">
                  <h3>{item.name}</h3>
                  {existing.length > 0 && <span className="pill warn">库中已有同名记录</span>}
                </div>
                <code className="path">{item.sourcePath}</code>
                {item.actualPath !== item.sourcePath && (
                  <code className="path" style={{ display: 'block' }}>
                    实际位置：{item.actualPath}
                  </code>
                )}
                {item.issues.map(i => (
                  <p className="form-hint" key={i}>
                    {message(i)}
                  </p>
                ))}
                {selected && existing.length > 0 && (
                  <label className="field" style={{ marginTop: 10 }}>
                    <span>库中有同名记录</span>
                    <select
                      aria-label={`${item.name} 同名处理`}
                      value={selected.action === 'replace' ? `replace:${selected.replaceId}` : selected.action}
                      onChange={e => {
                        const value = e.target.value;
                        setChoices({
                          ...choices,
                          [item.itemId]: {
                            ...selected,
                            action: value.startsWith('replace:') ? 'replace' : (value as 'skip' | 'new'),
                            replaceId: value.startsWith('replace:') ? value.slice(8) : null,
                          },
                        });
                      }}
                    >
                      <option value="skip">跳过（默认）</option>
                      <option value="new">另存为独立记录</option>
                      {existing.map(r => (
                        <option key={r.id} value={`replace:${r.id}`}>
                          替换 {r.displayName}（{r.id.slice(-8)}）
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>
            </div>
          );
        })}

        <p className="form-hint">
          导入只复制完整包到配置库，保留原来源文件。保存完成后，可另行选择 Agent 并预览分发。
        </p>
      </div>

      <footer className="dialog-footer">
        <span className="footer-copy">已选择 {actionable.length} 个完整包</span>
        <button type="button" className="button" onClick={onClose}>
          取消
        </button>
        <button
          type="button"
          className="button primary"
          data-action="preview-import"
          disabled={busy || !scan || !actionable.length}
          onClick={() => {
            if (scan) void onPreview(scan.scanId, Object.values(choices), source);
          }}
        >
          预览库变化
          <Icon name="arrow" size={16} />
        </button>
      </footer>
    </Dialog>
  );
}
