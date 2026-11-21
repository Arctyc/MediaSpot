import { useState } from "react";

export default function StarRating({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  const [hover, setHover] = useState(0);
  const shown = hover || value;
  return (
    <div className="stars" onMouseLeave={() => setHover(0)}>
      {Array.from({ length: 10 }, (_, i) => i + 1).map((n) => (
        <button
          key={n}
          className={`star ${n <= shown ? "filled" : ""} ${hover ? "previewing" : ""}`}
          onMouseEnter={() => setHover(n)}
          onClick={() => onChange(n)}
          title={`${n} / 10`}
          aria-label={`Rate ${n} out of 10`}
        >
          {n <= shown ? "★" : "☆"}
        </button>
      ))}
      <span className="rating-label">{value ? `${value}/10` : "Not rated"}</span>
      {value > 0 && (
        <button className="link small" onClick={() => onChange(0)}>Clear</button>
      )}
    </div>
  );
}
