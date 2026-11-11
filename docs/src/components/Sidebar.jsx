import { useEffect, useState } from "react";

import { actions, findEntry, getGroupEntries, pages } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";
import { ExperimentalIcon } from "./ExperimentalBadge";

function PageButton({ name, label, experimental }) {
  const { state, dispatch } = useDocumentation();
  return (
    <button
      type="button"
      className="page-button"
      aria-current={state.entryName === name}
      onClick={() => dispatch({ type: actions.entryChosen, entryName: name })}
    >
      {label ?? name}
      {experimental && (
        <span className="experimental-mark" title="Experimental and not stable">
          <ExperimentalIcon />
        </span>
      )}
    </button>
  );
}

function getToggleLabel(groupName, isOpen) {
  if (isOpen) return `Close ${groupName}`;
  return `Open ${groupName}`;
}

function SidebarGroup({ group, isOpen, onToggle, onOpen }) {
  const { state, dispatch } = useDocumentation();
  const entries = getGroupEntries(state.document, group.name);

  function showGroup() {
    onOpen(group.name);
    dispatch({ type: actions.entryChosen, entryName: group.name });
  }

  return (
    <div className="group">
      <div className="group-row">
        <button type="button" className="group-name" aria-current={state.entryName === group.name} onClick={showGroup}>
          {group.name}
          {group.experimental && (
            <span className="experimental-mark" title="Experimental and not stable">
              <ExperimentalIcon />
            </span>
          )}
        </button>
        <button
          type="button"
          className="group-toggle"
          aria-expanded={isOpen}
          aria-label={getToggleLabel(group.name, isOpen)}
          onClick={() => onToggle(group.name)}
        >
          <span className="chevron" aria-hidden="true" />
        </button>
      </div>
      {isOpen && (
        <div className="group-items">
          {entries.map((entry) => (
            <PageButton key={entry.name} name={entry.name} experimental={entry.experimental} />
          ))}
        </div>
      )}
    </div>
  );
}

function getCurrentGroup(document, pageName) {
  return findEntry(document, pageName)?.group ?? pageName;
}

export function Sidebar() {
  const { state } = useDocumentation();
  const [openGroups, setOpenGroups] = useState({});
  const currentGroup = getCurrentGroup(state.document, state.entryName);

  function openGroup(groupName) {
    setOpenGroups((groups) => ({ ...groups, [groupName]: true }));
  }

  function toggleGroup(groupName) {
    setOpenGroups((groups) => ({ ...groups, [groupName]: !groups[groupName] }));
  }

  useEffect(() => {
    openGroup(currentGroup);
  }, [currentGroup]);

  return (
    <nav className="sidebar" aria-label="Functions and types">
      <div className="group">
        <PageButton name={pages.overview} label="Overview" />
      </div>
      {state.document.groups.map((group) => (
        <SidebarGroup
          key={group.name}
          group={group}
          isOpen={Boolean(openGroups[group.name])}
          onToggle={toggleGroup}
          onOpen={openGroup}
        />
      ))}
      <div className="group">
        <PageButton name={pages.versions} label="Versions" />
      </div>
      <p className="author">authored by josoriom</p>
    </nav>
  );
}
