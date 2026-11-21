import { useState } from "react";
import { initials } from "../utils";

export default function Art({ src, title }: { src: string | null; title: string }) {
  const [failed, setFailed] = useState(false);
  if (!src || failed) {
    return (
      <div className="placeholder">
        <span>{initials(title)}</span>
      </div>
    );
  }
  return <img src={src} alt={title} loading="lazy" onError={() => setFailed(true)} />;
}
