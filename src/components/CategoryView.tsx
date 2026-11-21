import { useCallback, useEffect, useState } from "react";
import { api, Category, CATEGORY_META, Item, LibraryData, Section, SeriesSummary } from "../api";
import type { TvTab } from "../App";
import SearchBar from "./SearchBar";
import MediaCard from "./MediaCard";
import SeriesCard from "./SeriesCard";
import WatchNext from "./WatchNext";
import Upcoming from "./Upcoming";
import { SeriesPickerDialog } from "./SeriesDialogs";

interface Props {
  category: Category;
  reloadKey: number;
  onReload: () => void;
  onOpenItem: (id: number) => void;
  onOpenSeries: (id: number) => void;
  tvTab: TvTab;
  setTvTab: (t: TvTab) => void;
  tvSyncing: boolean;
}

export default function CategoryView({ category, reloadKey, onReload, onOpenItem, onOpenSeries, tvTab, setTvTab, tvSyncing }: Props) {
  const meta = CATEGORY_META[category];
  const [data, setData] = useState<LibraryData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshingAll, setRefreshingAll] = useState(false);
  const [grouping, setGrouping] = useState(false);
  const [pickerItem, setPickerItem] = useState<Item | null>(null);

  const load = useCallback(async () => {
    try {
      setData(await api.listLibrary(category));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [category]);

  useEffect(() => { load(); }, [load, reloadKey]);

  const isTv = category === "tv";
  const tab: TvTab = isTv ? tvTab : "library";

  async function refreshAll() {
    setRefreshingAll(true);
    setError(null);
    try {
      const s = await api.refreshAllTv(null);
      if (s.failed > 0) setError(`${s.failed} show(s) failed to refresh: ${s.errors.join("; ")}`);
      onReload();
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshingAll(false);
    }
  }

  async function autoGroup() {
    setGrouping(true);
    setError(null);
    try {
      const n = await api.autoGroupBooks();
      if (n === 0) setError("No new series detected. You can group books manually from a card's ⋯ menu.");
      onReload();
    } catch (e) {
      setError(String(e));
    } finally {
      setGrouping(false);
    }
  }

  const sectionOf = (sec: Section) => ({
    series: data?.series.filter((s) => s.section === sec) ?? [],
    items: data?.items.filter((i) => i.section === sec) ?? [],
  });

  return (
    <div className="view">
      <header className="view-header">
        <h1>{meta.icon} {meta.label}</h1>
        <SearchBar category={category} onAdded={onReload} />
        {isTv && (
          <div className="subtabs">
            <button className={`subtab ${tab === "library" ? "active" : ""}`} onClick={() => setTvTab("library")}>Library</button>
            <button className={`subtab ${tab === "next" ? "active" : ""}`} onClick={() => setTvTab("next")}>Watch Next</button>
            <button className={`subtab ${tab === "upcoming" ? "active" : ""}`} onClick={() => setTvTab("upcoming")}>Upcoming</button>
            <div className="spacer" />
            <button className="btn small" onClick={refreshAll} disabled={refreshingAll || tvSyncing}>
              {refreshingAll || tvSyncing ? "Refreshing shows…" : "⟳ Refresh all shows"}
            </button>
          </div>
        )}
        {category === "book" && (
          <div className="subtabs">
            <span className="muted small">Series are detected from Hardcover data and similar titles. Use a card's ⋯ menu to group manually.</span>
            <div className="spacer" />
            <button className="btn small" onClick={autoGroup} disabled={grouping}>
              {grouping ? "Detecting…" : "⧉ Auto-group series"}
            </button>
          </div>
        )}
      </header>

      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {tab === "library" && data && (
        <>
          {(["active", "done"] as Section[]).map((sec) => {
            const { series, items } = sectionOf(sec);
            return (
              <section className="library-section" key={sec}>
                <div className="section-header">
                  <h2>{meta.sections[sec]}</h2>
                  <span className="count">{series.length + items.length}</span>
                </div>
                {series.length + items.length === 0 ? (
                  <p className="empty">
                    {sec === "active"
                      ? `Nothing in ${meta.sections.active} yet. Use the search bar above to add ${meta.noun}.`
                      : `Nothing in ${meta.sections.done} yet.`}
                  </p>
                ) : (
                  <div className="card-grid">
                    {series.map((s: SeriesSummary) => (
                      <SeriesCard key={`s${s.id}`} series={s} onOpen={onOpenSeries} onChanged={onReload} onError={setError} />
                    ))}
                    {items.map((it) => (
                      <MediaCard
                        key={it.id}
                        item={it}
                        onOpen={onOpenItem}
                        onChanged={onReload}
                        onError={setError}
                        onAddToSeries={category === "book" ? setPickerItem : undefined}
                      />
                    ))}
                  </div>
                )}
              </section>
            );
          })}
        </>
      )}
      {tab === "next" && <WatchNext onOpen={onOpenItem} reloadKey={reloadKey} onChanged={onReload} />}
      {tab === "upcoming" && <Upcoming onOpen={onOpenItem} reloadKey={reloadKey} />}

      {pickerItem && (
        <SeriesPickerDialog
          item={pickerItem}
          onClose={() => setPickerItem(null)}
          onDone={() => { setPickerItem(null); onReload(); }}
        />
      )}
    </div>
  );
}