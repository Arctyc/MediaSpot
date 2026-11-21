import { useCallback, useEffect, useState } from "react";
import { api, EpisodeWithShow } from "../api";
import { epCode, epDate, fmtDate, relativeDays } from "../utils";
import Art from "./Art";

export default function Upcoming({ onOpen, reloadKey }: { onOpen: (id: number) => void; reloadKey: number }) {
  const [rows, setRows] = useState<EpisodeWithShow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try { setRows(await api.upcoming()); } catch (e) { setError(String(e)); }
  }, []);

  useEffect(() => { load(); }, [load, reloadKey]);

  if (error) return <div className="error-banner"><span>{error}</span></div>;
  if (!rows) return <p className="muted">Loading…</p>;
  if (rows.length === 0) return <p className="empty">No announced upcoming episodes for the shows you're watching.</p>;

  return (
    <div className="ep-feed">
      {rows.map((r) => {
        const date = epDate(r.episode);
        return (
          <div className="ep-card" key={r.episode.id}>
            <div className="ep-thumb" onClick={() => onOpen(r.showId)}><Art src={r.showImage} title={r.showTitle} /></div>
            <div className="ep-info">
              <button className="link show-link" onClick={() => onOpen(r.showId)}>{r.showTitle}</button>
              <div className="ep-title">
                <span className="ep-code">{epCode(r.episode.season, r.episode.number)}</span> {r.episode.name || "TBA"}
              </div>
              {r.remaining > 0 && <div className="muted small">+{r.remaining} more announced</div>}
            </div>
            <div className="ep-when">
              <div>{fmtDate(date)}</div>
              <div className="muted small">{relativeDays(date)}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
