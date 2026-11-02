import {
  Children,
  isValidElement,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { KeyboardEvent, ReactNode } from "react";
import "../styles/select.css";

/** Single-value select. Focus stays on the trigger; the popover sits above modal dialogs. */
export function Select({
  value,
  onChange,
  children,
  disabled = false,
  ...label
}: {
  value: string | number;
  onChange: (value: string) => void;
  children: ReactNode;
  disabled?: boolean;
  "aria-label": string;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const search = useRef({ text: "", time: 0 });
  const options = Children.toArray(children)
    .filter(isValidElement)
    .map((child) => {
      const props = child.props as {
        value: string | number;
        children: ReactNode;
        "data-description"?: string;
      };
      return {
        value: String(props.value),
        label: props.children,
        description: props["data-description"],
      };
    });
  const selected = options.findIndex(
    (option) => option.value === String(value),
  );
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const available = !disabled && options.length > 0;

  function show() {
    if (!available) return;
    setActive(Math.max(0, selected));
    setOpen(true);
  }
  function choose(index: number) {
    if (!options[index]) return;
    onChange(options[index].value);
    setOpen(false);
    trigger.current?.focus();
  }

  useLayoutEffect(() => {
    const popup = panel.current;
    const button = trigger.current;
    if (!open || !available || !popup || !button) return;
    popup.showPopover();
    const position = () => {
      const rect = button.getBoundingClientRect();
      const margin = 10,
        gap = 7;
      const below = window.innerHeight - rect.bottom - margin - gap;
      const above = rect.top - margin - gap;
      const height = Math.min(popup.scrollHeight + 2, 320);
      const upward = below < height && above > below;
      popup.dataset.side = upward ? "top" : "bottom";
      popup.style.width = `${Math.min(rect.width, window.innerWidth - margin * 2)}px`;
      popup.style.left = `${Math.max(margin, Math.min(rect.left, window.innerWidth - rect.width - margin))}px`;
      popup.style.maxHeight = `${Math.max(0, Math.min(320, upward ? above : below))}px`;
      popup.style.top = upward ? "auto" : `${rect.bottom + gap}px`;
      popup.style.bottom = upward
        ? `${window.innerHeight - rect.top + gap}px`
        : "auto";
    };
    position();
    const resize = new ResizeObserver(position);
    resize.observe(button);
    const scroll = (event: Event) => {
      if (!popup.contains(event.target as Node)) position();
    };
    const outside = (event: PointerEvent) => {
      if (
        !popup.contains(event.target as Node) &&
        !button.contains(event.target as Node)
      )
        setOpen(false);
    };
    window.addEventListener("resize", position);
    document.addEventListener("scroll", scroll, true);
    document.addEventListener("pointerdown", outside, true);
    return () => {
      popup.hidePopover();
      resize.disconnect();
      window.removeEventListener("resize", position);
      document.removeEventListener("scroll", scroll, true);
      document.removeEventListener("pointerdown", outside, true);
    };
  }, [open, available]);

  useEffect(() => {
    if (!available) setOpen(false);
  }, [available]);
  useEffect(() => {
    if (open)
      panel.current?.children[active]?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  function keyDown(event: KeyboardEvent<HTMLButtonElement>) {
    // Select keystrokes must not trigger score-editor shortcuts or dismiss its dialog.
    event.stopPropagation();
    if (event.key === "Tab") {
      setOpen(false);
      return;
    }
    if (event.key === "Escape" && open) {
      event.preventDefault();
      setOpen(false);
      return;
    }
    if (
      ["ArrowDown", "ArrowUp", "Home", "End", "Enter", " "].includes(event.key)
    ) {
      event.preventDefault();
      if (!open) {
        show();
        return;
      }
      if (event.key === "Enter" || event.key === " ") {
        choose(active);
        return;
      }
      setActive((index) =>
        event.key === "Home"
          ? 0
          : event.key === "End"
            ? options.length - 1
            : Math.max(
                0,
                Math.min(
                  options.length - 1,
                  index + (event.key === "ArrowDown" ? 1 : -1),
                ),
              ),
      );
    } else if (
      event.key.length === 1 &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey
    ) {
      const now = Date.now();
      search.current.text =
        now - search.current.time > 600
          ? event.key
          : search.current.text + event.key;
      search.current.time = now;
      const match = options.findIndex((option) =>
        String(option.label)
          .toLocaleLowerCase()
          .startsWith(search.current.text.toLocaleLowerCase()),
      );
      if (match >= 0) {
        if (!open) show();
        setActive(match);
      }
    }
  }

  return (
    <span className="select-control">
      <button
        {...label}
        ref={trigger}
        type="button"
        role="combobox"
        className="select-trigger"
        disabled={!available}
        aria-expanded={open && available}
        aria-haspopup="listbox"
        aria-controls={id}
        aria-activedescendant={open ? `${id}-${active}` : undefined}
        onClick={() => (open ? setOpen(false) : show())}
        onKeyDown={keyDown}
        onBlur={() => setOpen(false)}
      >
        <span className="select-value">
          {options[selected]?.label ?? "暂无可选项"}
        </span>
        <svg
          className="select-chevron"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.7"
          aria-hidden="true"
        >
          <path
            d="m7 10 5 5 5-5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>
      <div
        ref={panel}
        id={id}
        role="listbox"
        aria-label={label["aria-label"]}
        popover="manual"
        className="select-panel"
        onPointerDown={(event) => event.preventDefault()}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        {options.map((option, index) => (
          <div
            key={option.value}
            id={`${id}-${index}`}
            role="option"
            aria-selected={index === selected}
            className="select-option"
            data-active={index === active}
            onPointerMove={() => setActive(index)}
            onClick={() => choose(index)}
          >
            <span className="select-option-copy">
              <span>{option.label}</span>
              {option.description && <small>{option.description}</small>}
            </span>
          </div>
        ))}
      </div>
    </span>
  );
}
