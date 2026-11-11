import { useState } from "react";

function CopyIcon({ copied }) {
  return (
    <svg className="copy-icon" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      {copied ? (
        <path d="M3 8.5l3 3 7-7" />
      ) : (
        <>
          <rect x="5.5" y="5.5" width="8" height="8" rx="1.5" />
          <path d="M10.5 5.5V3.5a1.5 1.5 0 0 0-1.5-1.5H4a1.5 1.5 0 0 0-1.5 1.5v5A1.5 1.5 0 0 0 4 10h1.5" />
        </>
      )}
    </svg>
  );
}

export function CopyButton({ text, label }) {
  const [copied, setCopied] = useState(false);

  function copyText() {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    });
  }

  return (
    <button type="button" className="copy" onClick={copyText} aria-label={label} title={copied ? "Copied" : "Copy"}>
      <CopyIcon copied={copied} />
    </button>
  );
}
