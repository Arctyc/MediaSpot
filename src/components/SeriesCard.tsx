import { useCallback, useRef, useState } from "react";
import { api, SeriesSummary } from "../api";
import Art from "./Art";
import DropdownMenu from "./DropdownMenu";
import { TextPromptDialog } from "./SeriesDialogs";

interface Props {
  series: SeriesSummary;
  onOpen: (id: number) => void;
  onChanged: () => void;
  onError: (msg: string) => void;
}

export default function SeriesCard({ series, onOpen, onChanged, onError }: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [confirm, setConfirm] = useState<"ungroup" | "delete" | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [busy, setBusy] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);

  const closeMenu = useCallback(() => { setMenuOpen(false); setConfirm(null); }, []);

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

  const allRead = series.section === "done";

  return (
    <div className="card series-card" onClick={() => onOpen(series.id)}>
      <div className="card-art stacked">
        <Art src={series.imageUrl} title={series.title} />
        <span className={`card-badge ${allRead ? "subtle" : ""}`} title="Books read">
          {allRead ? "✓ All read" : `${series.readCount}/${series.bookCount} read`}
        </span>
        <span className="card-kind">SERIES</span>
        {series.bookCount > 0 && (
          <div className="card-progress"><div style={{ width: `${Math.round((series.readCount / series.bookCount) * 100)}%` }} /></div>
        )}
      </div>
      <div className="card-title" title={series.title}>{series.title}</div>
      <div className="card-sub">{series.bookCount} book{series.bookCount === 1 ? "" : "s"}{series.authors ? ` · ${series.authors}` : ""}</div>

      <div className="card-controls" onClick={(e) => e.stopPropagation()}>
        <button ref={btnRef} className={`card-menu-btn ${menuOpen ? "open" : ""}`} title="Actions"
          onClick={() => (menuOpen ? closeMenu() : setMenuOpen(true))}>⋯</button>
        <DropdownMenu anchorRef={btnRef} open={menuOpen} onClose={closeMenu}>
          <button onClick={() => { closeMenu(); onOpen(series.id); }}>Open series</button>
          <button disabled={busy} onClick={() => run(() => api.setSeriesRead(series.id, !allRead))}>
            {allRead ? "Mark all as unread" : "Mark all as read"}
          </button>
          <button onClick={() => { closeMenu(); setRenaming(true); }}>Rename series…</button>
          <hr />
          {confirm === "ungroup" ? (
            <button disabled={busy} onClick={() => run(() => api.ungroupSeries(series.id))}>Confirm: ungroup (keep books)</button>
          ) : (
            <button onClick={() => setConfirm("ungroup")}>Ungroup series</button>
          )}
          {confirm === "delete" ? (
            <button className="danger" disabled={busy} onClick={() => run(() => api.deleteSeries(series.id))}>
              Confirm: delete series and its {series.bookCount} books
            </button>
          ) : (
            <button className="danger" onClick={() => setConfirm("delete")}>Remove series and all its books</button>
          )}
        </DropdownMenu>
      </div>

      {renaming && (
        <TextPromptDialog
          title="Rename series"
          label="Series name"
          initial={series.title}
          onClose={() => setRenaming(false)}
          onSubmit={async (name) => { setRenaming(false); await run(() => api.renameSeries(series.id, name)); }}
        />
      )}
    </div>
  );
}