export function ExperimentalIcon() {
  return (
    <svg className="experimental-icon" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      <path d="M6 1.5h4M6.75 1.5v4.2L2.6 12.9a1.1 1.1 0 0 0 .95 1.6h8.9a1.1 1.1 0 0 0 .95-1.6L9.25 5.7V1.5M4.4 10h7.2" />
    </svg>
  );
}

export function ExperimentalBadge() {
  return (
    <span className="experimental" title="Experimental and not stable: it may change in any release.">
      <ExperimentalIcon />
      Experimental
    </span>
  );
}
