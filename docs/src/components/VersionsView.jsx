import { useDocumentation } from "../state/DocumentationContext";
import { CopyButton } from "./CopyButton";

export function VersionsView() {
  const { state } = useDocumentation();

  return (
    <>
      <div className="entry-heading">
        <h2>Versions</h2>
      </div>
      <p className="summary">Every release and the commit it came from.</p>
      <ul className="version-history">
        <li className="version-history-head" aria-hidden="true">
          <span>Version</span>
          <span>Date</span>
          <span>Message</span>
          <span>Commit</span>
        </li>
        {state.versions.map((item) => (
          <li key={item.version}>
            <span className="version-history-number">{item.version}</span>
            <span className="version-history-date">{item.date ?? "Not released yet"}</span>
            <span className="version-history-message">{item.message}</span>
            <span className="version-history-commit">
              {item.commit && (
                <>
                  <code>{item.commit.slice(0, 7)}</code>
                  <CopyButton text={item.commit} label={`Copy commit ${item.commit}`} />
                </>
              )}
            </span>
          </li>
        ))}
      </ul>
    </>
  );
}
