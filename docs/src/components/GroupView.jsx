import { getDisplayName, getFirstSentence } from "../names";
import { actions, getGroupEntries } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";
import { ExperimentalBadge } from "./ExperimentalBadge";
import { LinkedText } from "./LinkedText";
import { IonicText } from "./IonicText";

export function GroupView({ group }) {
  const { state, dispatch } = useDocumentation();
  const entries = getGroupEntries(state.document, group.name);

  return (
    <>
      <div className="entry-heading">
        <h2>{group.name}</h2>
        {group.experimental && <ExperimentalBadge />}
      </div>
      <p className="summary">
        <LinkedText text={group.description} />
      </p>
      {group.warning && (
        <p className="warning" role="note">
          <strong>Warning: </strong>
          <IonicText text={group.warning} />
        </p>
      )}
      <section className="block">
        <h3>In this group</h3>
        <ul className="group-list">
          {entries.map((entry) => (
            <li key={entry.name}>
              <button type="button" className="group-list-name" onClick={() => dispatch({ type: actions.entryChosen, entryName: entry.name })}>
                {getDisplayName(entry, state.language)}
              </button>
              <span className="group-list-summary">
                <IonicText text={getFirstSentence(entry.summary)} />
              </span>
            </li>
          ))}
        </ul>
      </section>
    </>
  );
}
