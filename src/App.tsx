import { useEffect, useRef, useState } from "react";
import { api, Category } from "./api";
import Sidebar from "./components/Sidebar";
import CategoryView from "./components/CategoryView";
import DetailsPage from "./components/DetailsPage";
import SeriesPage from "./components/SeriesPage";
import SettingsPage from "./components/SettingsPage";

export type TvTab = "library" | "next" | "upcoming";
export type Page = Category | "settings";

/** Simple navigation stack so "Back" always returns to where you came from. */
export type View =
  | { kind: "library"; category: Category }
  | { kind: "series"; id: number; category: Category }
  | { kind: "item"; id: number; category: Category }
  | { kind: "settings" };

export default function App() {
  const [stack, setStack] = useState<View[]>([{ kind: "library", category: "movie" }]);
  const [tvTab, setTvTab] = useState<TvTab>("library");
  const [reloadKey, setReloadKey] = useState(0);
  const [tvSyncing, setTvSyncing] = useState(false);
  const started = useRef(false);

  const current = stack[stack.length - 1];
  const bump = () => setReloadKey((k) => k + 1);
  const push = (v: View) => setStack((s) => [...s, v]);
  const back = () => setStack((s) => (s.length > 1 ? s.slice(0, -1) : s));
  const navigate = (p: Page) =>
    setStack([p === "settings" ? { kind: "settings" } : { kind: "library", category: p }]);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    setTvSyncing(true);
    api
      .refreshAllTv(12)
      .then((s) => { if (s.refreshed > 0) bump(); })
      .catch(() => {})
      .finally(() => setTvSyncing(false));
  }, []);

  const activePage: Page = current.kind === "settings" ? "settings" : current.category;

  let content;
  switch (current.kind) {
    case "settings":
      content = <SettingsPage />;
      break;
    case "library":
      content = (
        <CategoryView
          key={current.category}
          category={current.category}
          reloadKey={reloadKey}
          onReload={bump}
          onOpenItem={(id) => push({ kind: "item", id, category: current.category })}
          onOpenSeries={(id) => push({ kind: "series", id, category: current.category })}
          tvTab={tvTab}
          setTvTab={setTvTab}
          tvSyncing={tvSyncing}
        />
      );
      break;
    case "series":
      content = (
        <SeriesPage
          key={current.id}
          id={current.id}
          onBack={back}
          onOpenBook={(id) => push({ kind: "item", id, category: current.category })}
        />
      );
      break;
    case "item":
      content = (
        <DetailsPage
          key={current.id}
          id={current.id}
          onBack={back}
          onOpenSeries={(sid) => push({ kind: "series", id: sid, category: current.category })}
        />
      );
      break;
  }

  return (
    <div className="app">
      <Sidebar page={activePage} onNavigate={navigate} />
      <main className="main">{content}</main>
    </div>
  );
}