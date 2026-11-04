/**
 * A desktop-style context menu.
 *
 * Rendered inside the app (not in a portal) with fixed positioning so it can
 * overflow the scroll container. It closes on outside click, Escape, scroll and
 * resize, and clamps itself to the viewport so a right-click near an edge does
 * not push the menu off-screen.
 */
import { useEffect, useLayoutEffect, useRef, useState } from "react";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: string;
  /** Draw a divider above this item. */
  separator?: boolean;
  danger?: boolean;
  disabled?: boolean;
  shortcut?: string;
  onSelect: () => void;
}

interface Props {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

const EDGE_GAP = 8;

export default function ContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  // Measure after mount, then keep the menu inside the window.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const maxLeft = window.innerWidth - width - EDGE_GAP;
    const maxTop = window.innerHeight - height - EDGE_GAP;
    setPos({
      left: Math.max(EDGE_GAP, Math.min(x, maxLeft)),
      top: Math.max(EDGE_GAP, Math.min(y, maxTop)),
    });
  }, [x, y]);

  useEffect(() => {
    function onPointerDown(event: MouseEvent) {
      if (!ref.current?.contains(event.target as Node)) onClose();
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    }
    // Any scroll or resize invalidates the anchor point.
    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", onClose, true);
    window.addEventListener("resize", onClose);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", onClose, true);
      window.removeEventListener("resize", onClose);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="ctx-menu"
      role="menu"
      style={{ left: pos.left, top: pos.top }}
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item, index) => (
        <div key={item.id}>
          {item.separator && index > 0 && <div className="ctx-sep" role="separator" />}
          <button
            type="button"
            role="menuitem"
            className={`ctx-item${item.danger ? " danger" : ""}`}
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            <span className="ctx-icon" aria-hidden>
              {item.icon}
            </span>
            <span className="ctx-label">{item.label}</span>
            {item.shortcut && <span className="ctx-shortcut">{item.shortcut}</span>}
          </button>
        </div>
      ))}
    </div>
  );
}
