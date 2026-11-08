import { useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { Icon } from './Icon';

export function Dialog({
  title,
  onClose,
  locked = false,
  children,
  wide = false,
}: {
  title: string;
  onClose: () => void;
  locked?: boolean;
  children: ReactNode;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const action = useRef(onClose);
  action.current = onClose;

  useEffect(() => {
    const trigger = document.activeElement as HTMLElement | null;
    const dialog = ref.current!;
    dialog.showModal();
    return () => {
      dialog.close();
      if (trigger?.isConnected) trigger.focus();
    };
  }, []);

  return (
    <dialog
      ref={ref}
      className={wide ? 'dialog wide' : 'dialog'}
      aria-label={title}
      onCancel={e => {
        e.preventDefault();
        if (!locked) action.current();
      }}
    >
      <header className="dialog-head">
        <h2>{title}</h2>
        <button
          type="button"
          className="button quiet icon-button small"
          aria-label="关闭"
          disabled={locked}
          onClick={onClose}
        >
          <Icon name="close" size={16} />
        </button>
      </header>
      {children}
    </dialog>
  );
}
