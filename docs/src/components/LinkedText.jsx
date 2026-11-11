import { useMemo } from "react";

import { getDisplayName } from "../names";
import { actions, findEntry } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";
import { IonicText } from "./IonicText";

function getNamePattern(entries) {
  const names = [];
  for (const entry of entries) names.push(entry.name);
  names.sort((left, right) => right.length - left.length);
  return new RegExp(`\\b(${names.join("|")})\\b`);
}

export function LinkedText({ text }) {
  const { state, dispatch } = useDocumentation();
  const { entries } = state.document;
  const namePattern = useMemo(() => getNamePattern(entries), [entries]);
  const parts = text.split(namePattern);

  return parts.map((part, index) => {
    const isName = index % 2 === 1;
    if (!isName) return <IonicText key={index} text={part} />;
    const entry = findEntry(state.document, part);
    return (
      <button key={index} type="button" className="text-link" onClick={() => dispatch({ type: actions.entryChosen, entryName: part })}>
        {getDisplayName(entry, state.language)}
      </button>
    );
  });
}
