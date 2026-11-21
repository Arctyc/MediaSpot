import { useCallback, useEffect, useState } from "react";
import { api, EpisodeWithShow } from "../api";
import { epCode, epDate, fmtDate } from "../utils";
import Art from "./Art";

interface Props { onOpen: (id: number) => void; reloadKey: number; onChanged: () => void; }

export default function WatchNext({ onOpen, reloadKey, onChanged }: Props) {
  const [rows, setRows] = useState<EpisodeWithShow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  const load = useCallback(async () => {
    try { setRows(await api.watchNext()); } catch (e) { setError(String(e)); }
  }, []);

  useEffect(() => { load(); }, [load, reloadKey]);

  async function markWatched(r: EpisodeWithShow) {
    setBusyId(r.episode.id);
    try {
      await api.setEpisodeWatched(r.episode.id, true);
      await load();
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyId(null);
    }
  }

  if (error) return <div className="error-banner"><span>{error}</span></div>;
  if (!rows) return <p className="muted">Loading…</p>;
  if (rows.length === 0) return <p className="empty">🎉 You're all caught up on every show you're watching.</p>;

  return (
    <div className="ep-feed">
      {rows.map((r) => (
        <div className="ep-card" key={r.episode.id}>
          <div className="ep-thumb" onClick={() => onOpen(r.showId)}><Art src={r.showImage} title={r.showTitle} /></div>
          <div className="ep-info">
            <button className="link show-link" onClick={() => onOpen(r.showId)}>{r.showTitle}</button>
            <div className="ep-title">
              <span className="ep-code">{epCode(r.episode.season, r.episode.number)}</span> {r.episode.name}
            </div>
            <div className="muted small">
              Aired {fmtDate(epDate(r.episode))}
              {r.remaining > 0 && ` · ${r.remaining} more unwatched after this`}
            </div>
          </div>
          <button className="btn primary" disabled={busyId !== null} onClick={() => markWatched(r)}>
            {busyId === r.episode.id ? "Saving…" : "✓ Mark watched"}
          </button>
        </div>
      ))}
    </div>
  );
}
