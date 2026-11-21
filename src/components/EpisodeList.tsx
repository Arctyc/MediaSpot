import { useEffect, useMemo, useState } from "react";
import { api, Episode, Item } from "../api";
import { epDate, fmtDate } from "../utils";

interface Props {
  item: Item;
  episodes: Episode[];
  onShowUpdated: (item: Item) => Promise<void>;
  onEpisodesChanged: () => Promise<void>;
  onError: (msg: string) => void;
}

export default function EpisodeList({ item, episodes, onShowUpdated, onEpisodesChanged, onError }: Props) {
  const [pending, setPending] = useState(false);
  const [expanded, setExpanded] = useState<Set<number> | null>(null);

  // Seasons newest-first, episodes newest-first within each season.
  const seasons = useMemo(() => {
    const map = new Map<number, Episode[]>();
    for (const ep of episodes) {
      if (!map.has(ep.season)) map.set(ep.season, []);
      map.get(ep.season)!.push(ep);
    }
    return [...map.entries()]
      .sort((a, b) => b[0] - a[0])
      .map(([season, eps]) => ({ season, eps: [...eps].sort((a, b) => (b.number ?? 0) - (a.number ?? 0)) }));
  }, [episodes]);

  // Initially expand the season containing the next unwatched episode (or the latest aired season).
  useEffect(() => {
    if (expanded !== null || episodes.length === 0) return;
    const ordered = [...episodes].sort((a, b) => a.season - b.season || (a.number ?? 0) - (b.number ?? 0));
    const next = ordered.find((e) => e.aired && !e.watched);
    const latestAired = [...ordered].reverse().find((e) => e.aired);
    const open = next?.season ?? latestAired?.season ?? seasons[0]?.season;
    setExpanded(new Set(open !== undefined ? [open] : []));
  }, [episodes, expanded, seasons]);

  const isOpen = (s: number) => expanded?.has(s) ?? false;
  const toggleOpen = (s: number) =>
    setExpanded((prev) => {
      const n = new Set(prev ?? []);
      if (n.has(s)) n.delete(s);
      else n.add(s);
      return n;
    });

  async function act(fn: () => Promise<Item>) {
    setPending(true);
    try {
      await onShowUpdated(await fn());
    } catch (e) {
      onError(String(e));
    } finally {
      setPending(false);
    }
  }

  const unwatchedCount = episodes.filter((e) => e.aired && !e.watched).length;
  const maxSeason = seasons[0]?.season ?? 1;

  return (
    <section className="episodes">
      <div className="episodes-header">
        <h2>Seasons &amp; Episodes</h2>
        <span className="muted small">
          {seasons.length} season{seasons.length === 1 ? "" : "s"} · {episodes.length} episodes
        </span>
        <div className="spacer" />
        {unwatchedCount > 0 && (
          <button className="btn small" disabled={pending} onClick={() => act(() => api.markAllWatched(item.id))}>
            Mark all aired as watched
          </button>
        )}
      </div>

      {item.source === "manual" && (
        <AddEpisodes itemId={item.id} defaultSeason={Math.max(1, maxSeason)} onAdded={onEpisodesChanged} onError={onError} />
      )}

      {episodes.length === 0 ? (
        <p className="empty">
          No episodes yet. {item.source === "manual" ? "Add some above." : "Try the Refresh button."}
        </p>
      ) : (
        seasons.map(({ season, eps }) => {
          const aired = eps.filter((e) => e.aired);
          const watched = aired.filter((e) => e.watched).length;
          const all = aired.length > 0 && watched === aired.length;
          const some = watched > 0 && !all;
          const unaired = eps.length - aired.length;
          return (
            <div className="season" key={season}>
              <div className="season-header" onClick={() => toggleOpen(season)}>
                <input
                  type="checkbox"
                  checked={all}
                  disabled={pending || aired.length === 0}
                  ref={(el) => { if (el) el.indeterminate = some; }}
                  onClick={(e) => e.stopPropagation()}
                  onChange={(e) => act(() => api.setSeasonWatched(item.id, season, e.target.checked))}
                  title={all ? "Mark season unwatched" : "Mark all aired episodes in this season watched"}
                />
                <h3>Season {season}</h3>
                <span className="muted small">
                  {watched}/{aired.length} watched{unaired > 0 ? ` · ${unaired} upcoming` : ""}
                </span>
                <span className="spacer" />
                <span className={`chev ${isOpen(season) ? "open" : ""}`}>▸</span>
              </div>
              {isOpen(season) &&
                eps.map((ep) => {
                  const date = epDate(ep);
                  return (
                    <label
                      key={ep.id}
                      className={`episode-row ${ep.watched ? "watched" : ""} ${!ep.aired ? "unaired" : ""}`}
                      title={ep.summary ?? undefined}
                    >
                      <input
                        type="checkbox"
                        checked={ep.watched}
                        disabled={pending || (!ep.aired && !ep.watched)}
                        onChange={(e) => act(() => api.setEpisodeWatched(ep.id, e.target.checked))}
                      />
                      <span className="ep-num">E{ep.number ?? "?"}</span>
                      <span className="ep-name">{ep.name || `Episode ${ep.number ?? ""}`}</span>
                      <span className="ep-date">
                        {ep.aired ? (date ? fmtDate(date) : "") : `Airs ${fmtDate(date)}`}
                      </span>
                    </label>
                  );
                })}
            </div>
          );
        })
      )}
    </section>
  );
}

function AddEpisodes(props: {
  itemId: number;
  defaultSeason: number;
  onAdded: () => Promise<void>;
  onError: (m: string) => void;
}) {
  const [season, setSeason] = useState(props.defaultSeason);
  const [count, setCount] = useState(10);
  const [busy, setBusy] = useState(false);

  async function add() {
    setBusy(true);
    try {
      await api.addManualEpisodes(props.itemId, season, count);
      await props.onAdded();
    } catch (e) {
      props.onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="add-episodes">
      <span>Add episodes —</span>
      <label>Season <input type="number" min={0} value={season} onChange={(e) => setSeason(Number(e.target.value))} /></label>
      <label>Count <input type="number" min={1} max={500} value={count} onChange={(e) => setCount(Number(e.target.value))} /></label>
      <button className="btn small primary" disabled={busy} onClick={add}>{busy ? "Adding…" : "Add"}</button>
    </div>
  );
}
