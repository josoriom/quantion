import { actions } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";

function getTypeNames(entries) {
  const typeNames = new Set();
  for (const entry of entries) {
    if (entry.kind === "type") typeNames.add(entry.name);
  }
  return typeNames;
}

export function TypeText({ text }) {
  const { state, dispatch } = useDocumentation();
  const typeNames = getTypeNames(state.document.entries);
  const parts = text.split(/([A-Z][A-Za-z]+)/);
  return parts.map((part, index) =>
    typeNames.has(part) ? (
      <button key={index} type="button" className="type-link" onClick={() => dispatch({ type: actions.entryChosen, entryName: part })}>
        {part}
      </button>
    ) : (
      part
    ),
  );
}
