import { useState } from "react";
import { api, Category, CATEGORY_META, Section } from "../api";

interface Props {
  category: Category;
  initialTitle: string;
  seriesId?: number;
  onClose: () => void;
  onCreated: () => void;
}

export default function ManualAddDialog({ category, initialTitle, seriesId, onClose, onCreated }: Props) {
  const meta = CATEGORY_META[category];
  const [title, setTitle] = useState(initialTitle);
  const [subtitle, setSubtitle] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [releaseDate, setReleaseDate] = useState("");
  const [summary, setSummary] = useState("");
  const [section, setSection] = useState<Section>("active");
  const [status, setStatus] = useState("Running");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const orNull = (s: string) => (s.trim() ? s.trim() : null);

  async function submit() {
    if (!title.trim()) { setError("Title is required."); return; }
    setSaving(true);
    setError(null);
    try {
      await api.addManual({
        category,
        section,
        title: title.trim(),
        subtitle: orNull(subtitle),
        summary: orNull(summary),
        imageUrl: orNull(imageUrl),
        releaseDate: orNull(releaseDate),
        status: category === "tv" ? status : null,
        seriesId: seriesId ?? null,
      });
      onCreated();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  }

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Add {meta.label.replace(/s$/, "")} manually{seriesId ? " to this series" : ""}</h2>
        <label>Title *<input autoFocus value={title} onChange={(e) => setTitle(e.target.value)} /></label>
        <label>Subtitle<input value={subtitle} placeholder={meta.subtitleHint} onChange={(e) => setSubtitle(e.target.value)} /></label>
        <label>Cover image URL<input value={imageUrl} placeholder="https://…" onChange={(e) => setImageUrl(e.target.value)} /></label>
        <div className="row2">
          <label>Release date<input value={releaseDate} placeholder="YYYY-MM-DD" onChange={(e) => setReleaseDate(e.target.value)} /></label>
          <label>
            Add to
            <select value={section} onChange={(e) => setSection(e.target.value as Section)}>
              <option value="active">{seriesId ? "To read" : meta.sections.active}</option>
              <option value="done">{seriesId ? "Read" : meta.sections.done}</option>
            </select>
          </label>
          {category === "tv" && (
            <label>
              Series status
              <select value={status} onChange={(e) => setStatus(e.target.value)}>
                <option>Running</option>
                <option>Ended</option>
              </select>
            </label>
          )}
        </div>
        <label>Description<textarea rows={5} value={summary} onChange={(e) => setSummary(e.target.value)} /></label>
        {category === "tv" && <p className="muted small">You can add seasons/episodes from the show's details page after creating it.</p>}
        {error && <div className="search-msg error">{error}</div>}
        <div className="modal-actions">
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn primary" disabled={saving} onClick={submit}>{saving ? "Saving…" : "Add"}</button>
        </div>
      </div>
    </div>
  );
}