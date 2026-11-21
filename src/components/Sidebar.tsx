import { CATEGORIES, CATEGORY_META } from "../api";
import type { Page } from "../App";

export default function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (p: Page) => void }) {
  return (
    <nav className="sidebar">
      <div className="brand">
        <span className="brand-mark">◆</span> MediaSpot
      </div>
      {CATEGORIES.map((c) => (
        <button key={c} className={`nav-item ${page === c ? "active" : ""}`} onClick={() => onNavigate(c)}>
          <span className="nav-icon">{CATEGORY_META[c].icon}</span>
          {CATEGORY_META[c].label}
        </button>
      ))}
      <div className="spacer" />
      <button className={`nav-item ${page === "settings" ? "active" : ""}`} onClick={() => onNavigate("settings")}>
        <span className="nav-icon">⚙️</span>
        Settings
      </button>
    </nav>
  );
}