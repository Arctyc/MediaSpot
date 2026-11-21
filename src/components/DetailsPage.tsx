import { useCallback, useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, CATEGORY_META, Episode, Item, Section } from "../api";
import { fmtDate, fmtDateTime } from "../utils";
import Art from "./Art";
import StarRating from "./StarRating";
import EpisodeList from "./EpisodeList";

function tvStatusLine(item: Item): string | null {
  const t = item.tv;
  if (!t) return null;
  if (t.total === 0) return "No episode information yet.";
  if (t.fullyComplete) return "✅ Series finished — every episode watched.";
  const unwatched = t.aired - t.watchedAired;
  if (unwatched > 0) return `▶ ${unwatched} unwatched episode${unwatched === 1 ? "" : "s"} available.`;
  if (t.caughtUp) {
    return t.nextAirdate
      ? `⏳ Up to date — next episode airs ${fmtDate(t.nextAirdate)}.`
      : `⏳ Up to date — waiting on new episodes (status: ${item.status ?? "unknown"}).`;
  }
  return null;
}

interface Props {
  id: number;
  onBack: () => void;
  onOpenSeries: (seriesId: number) => void;
}

export default function DetailsPage({ id, onBack, onOpenSeries }: Props) {
  const [item, setItem] = useState<Item | null>(null);
  const [episodes, setEpisodes] = useState<Episode[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const loadEpisodes = useCallback(async () => {
    setEpisodes(await api.getEpisodes(id));
  }, [id]);

  const load = useCallback(async () => {
    try {
      const it = await api.getItem(id);
      setItem(it);
      if (it.category === "tv") await loadEpisodes();
    } catch (e) {
      setError(String(e));
    }
  }, [id, loadEpisodes]);

  useEffect(() => { load(); }, [load]);

  useEffect(() => {
    if (!notice) return;
    const t = window.setTimeout(() => setNotice(null), 6000);
    return () => window.clearTimeout(t);
  }, [notice]);

  if (!item) {
    return (
      <div className="view details-view">
        <div className="details-topbar"><button className="btn" onClick={onBack}>← Back</button></div>
        {error ? <div className="error-banner"><span>{error}</span></div> : <p className="muted">Loading…</p>}
      </div>
    );
  }

  const meta = CATEGORY_META[item.category];
  const other: Section = item.section === "active" ? "done" : "active";
  const facts = item.extra?.facts ?? [];
  const genres = item.extra?.genres ?? [];
  const links = item.extra?.links ?? [];
  const inSeries = item.category === "book" && item.seriesId != null;
  const sectionLabel = (s: Section) =>
    inSeries ? (s === "done" ? "Read" : "Unread") : meta.sections[s];

  function sectionChangeNotice(before: Section, after: Item) {
    if (before === after.section) return;
    setNotice(after.section === "done" ? `🎉 Moved to ${sectionLabel("done")}.` : `Moved back to ${sectionLabel("active")}.`);
  }

  async function applyShowUpdate(updated: Item) {
    if (item) sectionChangeNotice(item.section, updated);
    setItem(updated);
    await loadEpisodes();
  }

  async function rate(r: number) {
    try { setItem(await api.setRating(id, r)); } catch (e) { setError(String(e)); }
  }

  async function refresh() {
    if (!item) return;
    setBusy(true);
    setError(null);
    try {
      const before = item.section;
      const it = await api.refreshItem(id);
      setItem(it);
      if (it.category === "tv") await loadEpisodes();
      if (before !== it.section) sectionChangeNotice(before, it);
      else setNotice("Details refreshed.");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function move() {
    try { setItem(await api.setSection(id, other)); } catch (e) { setError(String(e)); }
  }

  async function remove() {
    try { await api.removeItem(id); onBack(); } catch (e) { setError(String(e)); }
  }

  async function open(url: string) {
    try { await openUrl(url); } catch (e) { setError(String(e)); }
  }

  const tvLine = item.category === "tv" ? tvStatusLine(item) : null;

  return (
    <div className="view details-view">
      <div className="details-topbar">
        <button className="btn" onClick={onBack}>← Back</button>
      </div>
      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}
      {notice && <div className="notice">{notice}</div>}

      <div className="details-top">
        <div className="details-art"><Art src={item.imageUrl} title={item.title} /></div>
        <div className="details-info">
          <div className="details-kicker">
            {meta.label} · <span className={`pill ${item.section}`}>{sectionLabel(item.section)}</span>
            {" · "}{item.source === "manual" ? "Manual entry" : `via ${meta.provider}`}
            {inSeries && (
              <>
                {" · "}Part of{" "}
                <button className="link" onClick={() => onOpenSeries(item.seriesId!)}>
                  {item.seriesTitle ?? "series"}
                </button>
                {item.seriesPosition != null && ` #${item.seriesPosition}`}
              </>
            )}
          </div>
          <h1 className="details-title">{item.title}</h1>
          {item.subtitle && <div className="details-sub">{item.subtitle}</div>}

          <StarRating value={item.rating} onChange={rate} />

          {genres.length > 0 && (
            <div className="chips">{genres.map((g) => <span className="chip" key={g}>{g}</span>)}</div>
          )}
          {tvLine && <div className="tv-status">{tvLine}</div>}

          {facts.length > 0 && (
            <dl className="facts">
              {facts.map((f, i) => (
                <div className="fact" key={i}><dt>{f.label}</dt><dd>{f.value}</dd></div>
              ))}
            </dl>
          )}

          {links.length > 0 && (
            <div className="links">
              {links.map((l) => (
                <button key={l.url} className="btn small link-btn" title={l.url} onClick={() => open(l.url)}>
                  ↗ {l.label}
                </button>
              ))}
            </div>
          )}

          <p className="summary">{item.summary || "No description available."}</p>

          <div className="details-actions">
            <button
              className="btn primary"
              onClick={refresh}
              disabled={busy || item.source === "manual"}
              title={item.source === "manual" ? "Manual entries have no online source" : "Fetch latest data"}
            >
              {busy ? "Refreshing…" : "⟳ Refresh"}
            </button>
            <button className="btn" onClick={move}>
              {inSeries ? (item.section === "done" ? "Mark as unread" : "Mark as read") : `Move to ${meta.sections[other]}`}
            </button>
            {confirmRemove ? (
              <>
                <button className="btn danger solid" onClick={remove}>Confirm remove</button>
                <button className="btn" onClick={() => setConfirmRemove(false)}>Cancel</button>
              </>
            ) : (
              <button className="btn danger" onClick={() => setConfirmRemove(true)}>Remove</button>
            )}
          </div>
          {item.lastRefreshed && <div className="muted small">Last refreshed {fmtDateTime(item.lastRefreshed)}</div>}
        </div>
      </div>

      {item.category === "tv" && (
        <EpisodeList
          item={item}
          episodes={episodes}
          onShowUpdated={applyShowUpdate}
          onEpisodesChanged={load}
          onError={(m) => setError(m)}
        />
      )}
    </div>
  );
}