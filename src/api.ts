import { invoke } from "@tauri-apps/api/core";

export type Category = "movie" | "tv" | "book" | "game";
export type Section = "active" | "done";

export interface Fact { label: string; value: string; }
export interface Link { label: string; url: string; }

export interface TvProgress {
  total: number;
  aired: number;
  watchedAired: number;
  unaired: number;
  seriesEnded: boolean;
  caughtUp: boolean;
  fullyComplete: boolean;
  nextAirdate: string | null;
}

export interface Item {
  id: number;
  category: Category;
  section: Section;
  source: string;
  externalId: string | null;
  title: string;
  subtitle: string | null;
  summary: string | null;
  imageUrl: string | null;
  releaseDate: string | null;
  status: string | null;
  extra: { facts?: Fact[]; genres?: string[]; links?: Link[]; seriesName?: string | null; seriesPosition?: number | null } | null;
  rating: number;
  addedAt: string;
  updatedAt: string;
  lastRefreshed: string | null;
  seriesId: number | null;
  seriesTitle: string | null;
  seriesPosition: number | null;
  tv: TvProgress | null;
}

export interface SeriesSummary {
  id: number;
  category: Category;
  title: string;
  section: Section;
  imageUrl: string | null;
  bookCount: number;
  readCount: number;
  authors: string | null;
  createdAt: string;
}

export interface SeriesDetail { series: SeriesSummary; books: Item[]; }
export interface LibraryData { items: Item[]; series: SeriesSummary[]; }

export interface Episode {
  id: number;
  itemId: number;
  externalId: string | null;
  season: number;
  number: number | null;
  name: string | null;
  airdate: string | null;
  airstamp: string | null;
  runtime: number | null;
  summary: string | null;
  imageUrl: string | null;
  watched: boolean;
  watchedAt: string | null;
  aired: boolean;
}

export interface EpisodeWithShow {
  episode: Episode;
  showId: number;
  showTitle: string;
  showImage: string | null;
  remaining: number;
}

export interface SearchResult {
  source: string;
  externalId: string;
  title: string;
  subtitle: string | null;
  summary: string | null;
  imageUrl: string | null;
  year: string | null;
  inLibrary: boolean;
}

export interface Settings {
  tmdbKey: string;
  igdbClientId: string;
  igdbClientSecret: string;
  hardcoverToken: string;
}

export interface ManualInput {
  category: Category;
  section: Section;
  title: string;
  subtitle: string | null;
  summary: string | null;
  imageUrl: string | null;
  releaseDate: string | null;
  status: string | null;
  seriesId: number | null;
}

export interface RefreshSummary { refreshed: number; failed: number; errors: string[]; }
export interface AppInfo { dbPath: string; version: string; }

export const CATEGORIES: Category[] = ["movie", "tv", "book", "game"];

export const CATEGORY_META: Record<
  Category,
  { label: string; noun: string; icon: string; provider: string; sections: Record<Section, string>; subtitleHint: string }
> = {
  movie: { label: "Movies", noun: "movies", icon: "🎬", provider: "TMDB", sections: { active: "To-Watch", done: "Watched" }, subtitleHint: "e.g. 2014 · Christopher Nolan" },
  tv: { label: "TV Shows", noun: "shows", icon: "📺", provider: "TVMaze", sections: { active: "Watching", done: "Completed" }, subtitleHint: "e.g. HBO · 2011–2019" },
  book: { label: "Books", noun: "books", icon: "📚", provider: "Hardcover", sections: { active: "Reading", done: "Completed" }, subtitleHint: "Author(s)" },
  game: { label: "Video Games", noun: "games", icon: "🎮", provider: "IGDB", sections: { active: "Playing", done: "Completed" }, subtitleHint: "e.g. Nintendo · Switch" },
};

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),

  listLibrary: (category: Category) => invoke<LibraryData>("list_library", { category }),
  getItem: (id: number) => invoke<Item>("get_item", { id }),
  search: (category: Category, query: string) => invoke<SearchResult[]>("search", { category, query }),
  addFromSearch: (category: Category, source: string, externalId: string, section: Section, seriesId: number | null = null) =>
    invoke<Item>("add_from_search", { category, source, externalId, section, seriesId }),
  addManual: (input: ManualInput) => invoke<Item>("add_manual", { input }),
  removeItem: (id: number) => invoke<void>("remove_item", { id }),
  setSection: (id: number, section: Section) => invoke<Item>("set_section", { id, section }),
  setRating: (id: number, rating: number) => invoke<Item>("set_rating", { id, rating }),
  refreshItem: (id: number) => invoke<Item>("refresh_item", { id }),

  // series (books)
  getSeries: (id: number) => invoke<SeriesDetail>("get_series", { id }),
  listSeries: (category: Category) => invoke<SeriesSummary[]>("list_series", { category }),
  createSeries: (category: Category, title: string, itemIds: number[]) =>
    invoke<SeriesSummary>("create_series", { category, title, itemIds }),
  renameSeries: (id: number, title: string) => invoke<SeriesSummary>("rename_series", { id, title }),
  addToSeries: (seriesId: number, itemIds: number[]) => invoke<SeriesSummary>("add_to_series", { seriesId, itemIds }),
  removeFromSeries: (itemId: number) => invoke<Item>("remove_from_series", { itemId }),
  ungroupSeries: (seriesId: number) => invoke<void>("ungroup_series", { seriesId }),
  deleteSeries: (seriesId: number) => invoke<void>("delete_series", { seriesId }),
  setSeriesRead: (seriesId: number, read: boolean) => invoke<SeriesSummary>("set_series_read", { seriesId, read }),
  autoGroupBooks: () => invoke<number>("auto_group_books"),

  // tv
  getEpisodes: (itemId: number) => invoke<Episode[]>("get_episodes", { itemId }),
  setEpisodeWatched: (episodeId: number, watched: boolean) => invoke<Item>("set_episode_watched", { episodeId, watched }),
  setSeasonWatched: (itemId: number, season: number, watched: boolean) =>
    invoke<Item>("set_season_watched", { itemId, season, watched }),
  markAllWatched: (itemId: number) => invoke<Item>("mark_all_watched", { itemId }),
  addManualEpisodes: (itemId: number, season: number, count: number) =>
    invoke<void>("add_manual_episodes", { itemId, season, count }),
  watchNext: () => invoke<EpisodeWithShow[]>("watch_next"),
  upcoming: () => invoke<EpisodeWithShow[]>("upcoming"),
  refreshAllTv: (staleHours: number | null) => invoke<RefreshSummary>("refresh_all_tv", { staleHours }),
};