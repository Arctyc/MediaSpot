import { useCallback, useEffect, useRef, useState } from "react";
import { api, Category, CATEGORY_META, SearchResult, Section } from "../api";
import Art from "./Art";
import ManualAddDialog from "./ManualAddDialog";

interface Props {
  category: Category;
  onAdded: () => void;
  /** When set, added books go straight into this series. */
  seriesId?: number;
  sectionLabels?: Record<Section, string>;
}

export default function SearchBar({ category, onAdded, seriesId, sectionLabels }: Props) {
  const meta = CATEGORY_META[category];
  const labels = sectionLabels ?? meta.sections;
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [adding, setAdding] = useState<string | null>(null);
  const [showManual, setShowManual] = useState(false);
  const reqId = useRef(0);
  const wrapRef = useRef<HTMLDivElement>(null);

  const runSearch = useCallback(
    async (q: string) => {
      const id = ++reqId.current;
      const trimmed = q.trim();
      if (trimmed.length < 2) {
        setResults(null);
        setError(null);
        setLoading(false);
        return;
      }
      setLoading(true);
      setError(null);
      try {
        const r = await api.search(category, trimmed);
        if (id === reqId.current) { setResults(r); setOpen(true); }
      } catch (e) {
        if (id === reqId.current) { setResults(null); setError(String(e)); setOpen(true); }
      } finally {
        if (id === reqId.current) setLoading(false);
      }
    },
    [category]
  );

  useEffect(() => {
    const t = window.setTimeout(() => runSearch(query), 450);
    return () => window.clearTimeout(t);
  }, [query, runSearch]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, []);

  async function add(r: SearchResult, section: Section) {
    setAdding(`${r.source}:${r.externalId}`);
    setError(null);
    try {
      await api.addFromSearch(category, r.source, r.externalId, section, seriesId ?? null);
      setResults((prev) =>
        prev?.map((x) => (x.source === r.source && x.externalId === r.externalId ? { ...x, inLibrary: true } : x)) ?? null
      );
      onAdded();
    } catch (e) {
      setError(String(e));
    } finally {
      setAdding(null);
    }
  }

  return (
    <div className="search-wrap" ref={wrapRef}>
      <div className="search-row">
        <div className="search-input">
          <span className="muted">🔍</span>
          <input
            value={query}
            placeholder={seriesId ? `Search ${meta.provider} to add books to this series…` : `Search ${meta.noun} on ${meta.provider}…`}
            onChange={(e) => { setQuery(e.target.value); setOpen(true); }}
            onFocus={() => setOpen(true)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setOpen(false);
              if (e.key === "Enter") runSearch(query);
            }}
          />
          {loading && <span className="spinner" />}
          {query && (
            <button className="clear-btn" onClick={() => { setQuery(""); setResults(null); setError(null); }}>✕</button>
          )}
        </div>
        <button className="btn" onClick={() => { setOpen(false); setShowManual(true); }}>+ Add manually</button>
      </div>

      {open && (results || error) && (
        <div className="search-results">
          {error && <div className="search-msg error">{error}</div>}
          {results && results.length === 0 && (
            <div className="search-msg">
              No results.{" "}
              <button className="link" onClick={() => { setOpen(false); setShowManual(true); }}>Add it manually</button>
            </div>
          )}
          {results?.map((r) => {
            const key = `${r.source}:${r.externalId}`;
            // Inside a series page, an already-owned book can still be pulled into the series.
            const canAdd = !r.inLibrary || seriesId != null;
            return (
              <div className="result-row" key={key}>
                <div className="result-thumb"><Art src={r.imageUrl} title={r.title} /></div>
                <div className="result-info">
                  <div className="result-title">
                    {r.title} {r.year && <span className="muted">({r.year})</span>}
                  </div>
                  {r.subtitle && <div className="result-sub">{r.subtitle}</div>}
                  {r.summary && <div className="result-summary">{r.summary}</div>}
                </div>
                <div className="result-actions">
                  {!canAdd ? (
                    <span className="badge">✓ In library</span>
                  ) : (
                    <>
                      {r.inLibrary && <span className="badge">In library</span>}
                      <button className="btn small primary" disabled={adding !== null} onClick={() => add(r, "active")}>
                        {adding === key ? "Adding…" : `+ ${labels.active}`}
                      </button>
                      <button className="btn small" disabled={adding !== null} onClick={() => add(r, "done")}>
                        + {labels.done}
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {showManual && (
        <ManualAddDialog
          category={category}
          initialTitle={query}
          seriesId={seriesId}
          onClose={() => setShowManual(false)}
          onCreated={() => { setShowManual(false); onAdded(); }}
        />
      )}
    </div>
  );
}