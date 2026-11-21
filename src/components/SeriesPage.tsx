import { useCallback, useEffect, useState } from "react";
import { api, SeriesDetail } from "../api";
import SearchBar from "./SearchBar";
import MediaCard from "./MediaCard";
import { AddExistingBooksDialog, TextPromptDialog } from "./SeriesDialogs";

interface Props {
  id: number;
  onBack: () => void;
  onOpenBook: (id: number) => void;
}

export default function SeriesPage({ id, onBack, onOpenBook }: Props) {
  const [data, setData] = useState<SeriesDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [gone, setGone] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [addingExisting, setAddingExisting] = useState(false);
  const [confirmUngroup, setConfirmUngroup] = useState(false);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setData(await api.getSeries(id));
      setError(null);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Series not found")) setGone(true);
      else setError(msg);
    }
  }, [id]);

  useEffect(() => { load(); }, [load]);

  async function run(fn: () => Promise<unknown>) {
    setBusy(true);
    try {
      await fn();
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (gone) {
    return (
      <div className="view">
        <div className="details-topbar"><button className="btn" onClick={onBack}>← Back</button></div>
        <p className="empty">This series no longer exists (it had no books left, so it was removed).</p>
      </div>
    );
  }
  if (!data) {
    return (
      <div className="view">
        <div className="details-topbar"><button className="btn" onClick={onBack}>← Back</button></div>
        {error ? <div className="error-banner"><span>{error}</span></div> : <p className="muted">Loading…</p>}
      </div>
    );
  }

  const { series, books } = data;
  const allRead = series.section === "done";

  return (
    <div className="view">
      <header className="view-header">
        <div className="details-topbar"><button className="btn" onClick={onBack}>← Back to Books</button></div>
        <div className="series-header">
          <div>
            <div className="details-kicker">Book series · <span className={`pill ${series.section}`}>{allRead ? "Completed" : "Reading"}</span></div>
            <h1 className="details-title">
              {series.title}
              <button className="icon-btn" title="Rename" onClick={() => setRenaming(true)}>✎</button>
            </h1>
            <div className="muted">
              {series.bookCount} book{series.bookCount === 1 ? "" : "s"} · {series.readCount} read
              {series.authors ? ` · ${series.authors}` : ""}
            </div>
          </div>
          <div className="spacer" />
          <div className="details-actions">
            <button className="btn" disabled={busy} onClick={() => run(() => api.setSeriesRead(series.id, !allRead))}>
              {allRead ? "Mark all unread" : "Mark all read"}
            </button>
            <button className="btn" onClick={() => setAddingExisting(true)}>+ Add from library</button>
            {confirmUngroup ? (
              <>
                <button className="btn danger solid" disabled={busy} onClick={() => run(async () => { await api.ungroupSeries(series.id); onBack(); })}>
                  Confirm ungroup
                </button>
                <button className="btn" onClick={() => setConfirmUngroup(false)}>Cancel</button>
              </>
            ) : (
              <button className="btn danger" onClick={() => setConfirmUngroup(true)} title="Books go back to the library as individual entries">
                Ungroup
              </button>
            )}
          </div>
        </div>
        <SearchBar
          category="book"
          seriesId={series.id}
          sectionLabels={{ active: "To read", done: "Read" }}
          onAdded={load}
        />
      </header>

      {error && (
        <div className="error-banner"><span>{error}</span><button onClick={() => setError(null)}>✕</button></div>
      )}

      <section className="library-section">
        <div className="section-header">
          <h2>Books in this series</h2>
          <span className="count">{books.length}</span>
          <span className="muted small">Ordered by series position</span>
        </div>
        {books.length === 0 ? (
          <p className="empty">No books yet. Search above to add some, or add from your library.</p>
        ) : (
          <div className="card-grid">
            {books.map((b) => (
              <MediaCard key={b.id} item={b} mode="series" onOpen={onOpenBook} onChanged={load} onError={setError} />
            ))}
          </div>
        )}
      </section>

      {renaming && (
        <TextPromptDialog
          title="Rename series"
          label="Series name"
          initial={series.title}
          onClose={() => setRenaming(false)}
          onSubmit={(name) => { setRenaming(false); run(() => api.renameSeries(series.id, name)); }}
        />
      )}
      {addingExisting && (
        <AddExistingBooksDialog
          seriesId={series.id}
          onClose={() => setAddingExisting(false)}
          onDone={() => { setAddingExisting(false); load(); }}
        />
      )}
    </div>
  );
}