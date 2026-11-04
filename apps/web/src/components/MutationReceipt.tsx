import type { Receipt } from '../types';
import { Status } from './Status';
import { message } from '../messages';
export function incompleteReceipt(receipt: Receipt): boolean {
  return !Array.isArray(receipt.outcomes) || receipt.outcomes.some(o => !['written', 'unchanged'].includes(o.outcome));
}
export function MutationReceipt({receipt}: {receipt: Receipt | null}) {
  if (!receipt) return null;
  return <section data-mutation-receipt aria-label="本次操作回执" className="stack"><p className="notice warning" role="alert">本次操作包含未完成项。输入已保留，请逐组核对；有需要恢复的事务时，请在变更历史重新读取并预览恢复。</p>{receipt.backupLocation && <p className="path">备份：{receipt.backupLocation}</p>}{receipt.outcomes.map((o, i) => <div className="receipt-row" key={i}><span className="path">{o.path}</span><span><Status state={o.outcome}/>{o.reason && <p className="form-hint">{message(o.reason)}</p>}</span></div>)}{receipt.operationId && <p className="path">操作：{receipt.operationId}</p>}</section>;
}
