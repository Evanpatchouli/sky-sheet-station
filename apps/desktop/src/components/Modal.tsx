import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { Icon } from "./Icon.tsx";
export function Modal({
  title,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      className={wide ? "analysis-dialog" : undefined}
      ref={dialog}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="dialog-title">
        <h2>{title}</h2>
        <button aria-label="关闭" onClick={onClose}>
          <Icon name="close" />
        </button>
      </div>
      {children}
    </dialog>
  );
}
