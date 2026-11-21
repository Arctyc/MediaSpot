import { ReactNode, useCallback, useRef, useState } from "react";
import { api, CATEGORY_META, Item, Section } from "../api";
import Art from "./Art";
import DropdownMenu from "./DropdownMenu";

interface Props {
  item: Item;
  /** "library": normal card. "series": card shown inside a book series page. */
  mode?: "library" | "series";
  onOpen: (id: number) => void;
  onChanged: () => void;
  onError: (msg: string) => void;
  onAddToSeries?: (item: Item) => void;
}

export default function MediaCard({ item, mode = "library", onOpen, onChanged, onError, onAddToSeries }: Props) {
  const meta = CATEGORY_META[item.category];
  const [menuOpen, setMenuOpen] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [busy, setBusy] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);

  const closeMenu = useCallback(() => { setMenuOpen(false); setConfirmRemove(false); }, []);
  const other: Section = item.section === "active" ? "done" : "active";

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await fn();
      onChanged();
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
      closeMenu();
    }
  };

  const tv = item.tv;
  const unwatched = tv ? tv.aired - tv.watchedAired : 0;
  let badge: ReactNode = null;
  if (tv && item.section === "active") {
    if (unwatched > 0) badge = <span className="card-badge" title="Unwatched episodes">{unwatched}</span>;
    else if (tv.caughtUp && !tv.fullyComplete) badge = <span className="card-badge subtle">Up to date</span>;
  }
  if (mode === "series") {
    badge = item.section === "done"
      ? <span className="card-badge subtle">✓ Read</span>
      : <span className="card-badge outline">Unread</span>;
  }
  const pct = tv && tv.aired > 0 ? Math.round((tv.watchedAired / tv.aired) * 100) : 0;
  const position = item.seriesPosition;

  return (
    <div className="card" onClick={() => onOpen(item.id)}>
      <div className="card-art">
        <Art src={item.imageUrl} title={item.title} />
        {badge}
        {mode === "series" && position != null && (
          <span className="card-position" title="Position in series">#{Number.isInteger(position) ? position : position.toFixed(1)}</span>
        )}
        {item.rating > 0 && <span className="card-rating">★ {item.rating}</span>}
        {tv && tv.aired > 0 && item.section === "active" && (
          <div className="card-progress"><div style={{ width: `${pct}%` }} /></div>
        )}
      </div>
      <div className="card-title" title={item.title}>{item.title}</div>
      {item.subtitle && <div className="card-sub">{item.subtitle}</div>}

      <div className="card-controls" onClick={(e) => e.stopPropagation()}>
        <button
          ref={btnRef}
          className={`card-menu-btn ${menuOpen ? "open" : ""}`}
          title="Actions"
          onClick={() => (menuOpen ? closeMenu() : setMenuOpen(true))}
        >
          ⋯
        </button>
        <DropdownMenu anchorRef={btnRef} open={menuOpen} onClose={closeMenu}>
          <button onClick={() => { closeMenu(); onOpen(item.id); }}>Open details</button>

          {mode === "series" ? (
            <button disabled={busy} onClick={() => run(() => api.setSection(item.id, other))}>
              {item.section === "done" ? "Mark as unread" : "Mark as read"}
            </button>
          ) : (
            <button disabled={busy} onClick={() => run(() => api.setSection(item.id, other))}>
              Move to {meta.sections[other]}
            </button>
          )}

          {item.category === "tv" && unwatched > 0 && (
            <button disabled={busy} onClick={() => run(() => api.markAllWatched(item.id))}>
              Mark all aired episodes watched
            </button>
          )}

          {item.category === "book" && mode === "library" && item.seriesId == null && onAddToSeries && (
            <button onClick={() => { closeMenu(); onAddToSeries(item); }}>Add to series…</button>
          )}
          {mode === "series" && (
            <button disabled={busy} onClick={() => run(() => api.removeFromSeries(item.id))}>Remove from series</button>
          )}

          {item.source !== "manual" && (
            <button disabled={busy} onClick={() => run(() => api.refreshItem(item.id))}>
              {busy ? "Working…" : "Refresh info"}
            </button>
          )}
          <hr />
          {confirmRemove ? (
            <button className="danger" disabled={busy} onClick={() => run(() => api.removeItem(item.id))}>
              Click again to confirm removal
            </button>
          ) : (
            <button className="danger" onClick={() => setConfirmRemove(true)}>Remove from library</button>
          )}
        </DropdownMenu>
      </div>
    </div>
  );
}