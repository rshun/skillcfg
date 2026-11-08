import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Dialog } from './Dialog';
type Ask = (message: string) => Promise<boolean>;
const Context = createContext<Ask>(async () => false);
export const useDiscardConfirmation = () => useContext(Context);
export function DiscardConfirmation({children}: {children: ReactNode}) {
  const [message, setMessage] = useState<string | null>(null);
  const pending = useRef<((decision: boolean) => void) | null>(null);
  const ask = useCallback<Ask>(text => {
    if (pending.current) return Promise.resolve(false);
    return new Promise(resolve => { pending.current = resolve; setMessage(text); });
  }, []);
  const finish = useCallback((decision: boolean) => {
    const resolve = pending.current; pending.current = null; setMessage(null); resolve?.(decision);
  }, []);
  useEffect(() => () => { pending.current?.(false); pending.current = null; }, []);
  return <Context.Provider value={ask}>{children}{message !== null && <Dialog title="放弃未保存的修改？" onClose={() => finish(false)}><div className="dialog-body"><p>{message}</p><p className="muted">继续编辑会保留当前输入。</p></div><footer className="dialog-footer"><button type="button" className="button" data-action="discard-cancel" autoFocus onClick={() => finish(false)}>继续编辑</button><button type="button" className="button danger" data-action="discard-confirm" onClick={() => finish(true)}>放弃修改</button></footer></Dialog>}</Context.Provider>;
}
