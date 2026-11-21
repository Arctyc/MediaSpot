import { useEffect, useState } from "react";
import { api, Item, SeriesSummary } from "../api";

/** Guess a series name from a book title: text before ":" / "(" / " - ". */
export function suggestSeriesName(title: string): string {
  const cut = title.split(/[:(\u2013\u2014]| - /)[0].trim();
  return cut || title;
}

export function TextPromptDialog(props: {
  title: string;
  label: string;
  initial: string;
  onClose: () => void;
  onSubmit: (value: string) => void | Promise<void>;
}) {
  const [value, setValue] = useState(props.initial);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => { e.stopPropagation(); props.onClose(); }} onClick={(e) => e.stopPropagation()}>
      <div className="modal small" onMouseDown={(e) => e.stopPropagation()}>
        <h2>{props.title}</h2>
        <label>{props.label}
          <input autoFocus value={value} onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter" && value.trim()) props.onSubmit(value.trim()); }} />
        </label>
        <div className="modal-actions">
          <button className="btn" onClick={props.onClose}>Cancel</button>
          <button className="btn primary" disabled={!value.trim()} onClick={() => props.onSubmit(value.trim())}>Save</button>
        </div>
      </div>
    </div>
  );
}

/** Put one ungrouped book into an existing series or a new one. */
export function SeriesPickerDialog(props: { item: Item; onClose: () => void; onDone: () => void }) {
  const { item } = props;
  const [series, setSeries] = useState<SeriesSummary[] | null>(null);
  const [choice, setChoice] = useState<"new" | number>("new");
  const [newName, setNewName] = useState(item.extra?.seriesName || suggestSeriesName(item.title));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listSeries("book").then((s) => {
      setSeries(s);
      const match = s.find((x) => x.title.toLowerCase() === newName.toLowerCase());
      if (match) setChoice(match.id);
    }).catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      if (choice === "new") await api.createSeries("book", newName.trim(), [item.id]);
      else await api.addToSeries(choice, [item.id]);
      props.onDone();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onMouseDown={props.onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Add “{item.title}” to a series</h2>
        {!series ? <p className="muted">Loading…</p> : (
          <div className="check-list">
            <label className="check-row">
              <input type="radio" checked={choice === "new"} onChange={() => setChoice("new")} />
              <span>Create new series:</span>
              <input className="inline" value={newName} onChange={(e) => setNewName(e.target.value)} onFocus={() => setChoice("new")} />
            </label>
            {series.map((s) => (
              <label className="check-row" key={s.id}>
                <input type="radio" checked={choice === s.id} onChange={() => setChoice(s.id)} />
                <span>{s.title}</span>
                <span className="muted small">{s.bookCount} book{s.bookCount === 1 ? "" : "s"}</span>
              </label>
            ))}
          </div>
        )}
        {error && <div className="search-msg error">{error}</div>}
        <div className="modal-actions">
          <button className="btn" onClick={props.onClose}>Cancel</button>
          <button className="btn primary" disabled={busy || (choice === "new" && !newName.trim())} onClick={submit}>
            {busy ? "Saving…" : "Add to series"}
          </button>
        </div>
      </div>
    </div>
  );
}

/** Pick ungrouped library books to add to a series. */
export function AddExistingBooksDialog(props: { seriesId: number; onClose: () => void; onDone: () => void }) {
  const [books, setBooks] = useState<Item[] | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listLibrary("book").then((d) => setBooks(d.items)).catch((e) => setError(String(e)));
  }, []);

  const toggle = (id: number) =>
    setSelected((prev) => { const n = new Set(prev); if (n.has(id)) n.delete(id); else n.add(id); return n; });

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      await api.addToSeries(props.seriesId, [...selected]);
      props.onDone();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  const shown = (books ?? []).filter((b) =>
    !filter.trim() || b.title.toLowerCase().includes(filter.toLowerCase()) || (b.subtitle ?? "").toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="modal-backdrop" onMouseDown={props.onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Add books from your library</h2>
        <input placeholder="Filter…" value={filter} onChange={(e) => setFilter(e.target.value)} />
        {!books ? <p className="muted">Loading…</p> : shown.length === 0 ? (
          <p className="muted">No ungrouped books{filter ? " match that filter" : " in your library"}.</p>
        ) : (
          <div className="check-list tall">
            {shown.map((b) => (
              <label className="check-row" key={b.id}>
                <input type="checkbox" checked={selected.has(b.id)} onChange={() => toggle(b.id)} />
                <span>{b.title}</span>
                {b.subtitle && <span className="muted small">{b.subtitle}</span>}
              </label>
            ))}
          </div>
        )}
        {error && <div className="search-msg error">{error}</div>}
        <div className="modal-actions">
          <button className="btn" onClick={props.onClose}>Cancel</button>
          <button className="btn primary" disabled={busy || selected.size === 0} onClick={submit}>
            {busy ? "Adding…" : `Add ${selected.size || ""} selected`}
          </button>
        </div>
      </div>
    </div>
  );
}