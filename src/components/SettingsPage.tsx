import { useEffect, useState } from "react";
import { api, AppInfo, Settings } from "../api";

const FIELDS: { key: keyof Settings; label: string; help: string; secret: boolean }[] = [
  { key: "tmdbKey", label: "TMDB API Key or Read Access Token", help: "themoviedb.org → Settings → API", secret: true },
  { key: "igdbClientId", label: "IGDB (Twitch) Client ID", help: "dev.twitch.tv/console/apps → your app", secret: false },
  { key: "igdbClientSecret", label: "IGDB (Twitch) Client Secret", help: "dev.twitch.tv/console/apps → New Secret", secret: true },
  { key: "hardcoverToken", label: "Hardcover API Token", help: "hardcover.app/account/api", secret: true },
];

export default function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [reveal, setReveal] = useState<Record<string, boolean>>({});
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => setError(String(e)));
    api.appInfo().then(setInfo).catch(() => {});
  }, []);

  async function save() {
    if (!settings) return;
    setError(null);
    try {
      await api.saveSettings(settings);
      setStatus("Saved ✓");
      window.setTimeout(() => setStatus(null), 2500);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="view settings">
      <header className="view-header"><h1>⚙️ Settings</h1></header>
      {error && <div className="error-banner"><span>{error}</span></div>}
      {!settings ? (
        <p className="muted">Loading…</p>
      ) : (
        <div className="settings-card">
          <h2>API credentials</h2>
          <p className="muted small">Stored locally in your MediaSpot database. TVMaze needs no key.</p>
          {FIELDS.map((f) => (
            <label key={f.key} className="setting-field">
              <span className="setting-label">{f.label}</span>
              <div className="setting-input">
                <input
                  type={f.secret && !reveal[f.key] ? "password" : "text"}
                  value={settings[f.key]}
                  onChange={(e) => setSettings({ ...settings, [f.key]: e.target.value })}
                  spellCheck={false}
                  autoComplete="off"
                />
                {f.secret && (
                  <button type="button" className="btn small" onClick={() => setReveal({ ...reveal, [f.key]: !reveal[f.key] })}>
                    {reveal[f.key] ? "Hide" : "Show"}
                  </button>
                )}
              </div>
              <span className="muted small">{f.help}</span>
            </label>
          ))}
          <div className="settings-actions">
            <button className="btn primary" onClick={save}>Save</button>
            {status && <span className="ok">{status}</span>}
          </div>
        </div>
      )}

      {info && (
        <div className="settings-card">
          <h2>About</h2>
          <p className="small">MediaSpot v{info.version}</p>
          <p className="small muted">Database: <code>{info.dbPath}</code></p>
          <p className="small muted">
            This product uses the TMDB API but is not endorsed or certified by TMDB. TV data provided by TVMaze
            (CC BY-SA). Book data from Hardcover. Game data from IGDB.
          </p>
        </div>
      )}
    </div>
  );
}
