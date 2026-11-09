import { useEffect, useMemo, useRef, useState } from "react";
import { levenshteinDistance } from "str-utilities";

import { getDisplayName, getFirstSentence } from "../names";
import { actions, pages } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";

const maxResults = 8;
const ranks = { exact: 0, start: 1, contains: 2, text: 3, typo: 4 };

function toKey(text) {
  return text.toLowerCase().replace(/[\s_\-]/g, "");
}

function getItems(document, language) {
  const items = [
    { target: pages.overview, title: "Overview", label: "Page", text: document.overview.summary },
    { target: pages.versions, title: "Versions", label: "Page", text: "Every release and its changes." },
  ];
  for (const group of document.groups) {
    items.push({ target: group.name, title: group.name, label: "Group", text: group.description });
  }
  for (const entry of document.entries) {
    items.push({ target: entry.name, title: getDisplayName(entry, language), label: entry.group, text: entry.summary });
  }
  for (const item of items) {
    item.key = toKey(item.title);
    item.lowerText = `${item.label} ${item.text}`.toLowerCase();
  }
  return items;
}

function getTypoLimit(query) {
  return Math.max(1, Math.floor(query.length / 3));
}

function getRank(item, query, words) {
  if (item.key === query) return { rank: ranks.exact, distance: 0 };
  if (item.key.startsWith(query)) return { rank: ranks.start, distance: item.key.length - query.length };
  if (item.key.includes(query)) return { rank: ranks.contains, distance: item.key.length - query.length };
  if (words.every((word) => item.lowerText.includes(word))) return { rank: ranks.text, distance: 0 };
  const distance = Math.min(
    levenshteinDistance(query, item.key),
    levenshteinDistance(query, item.key.slice(0, query.length)),
  );
  if (distance <= getTypoLimit(query)) return { rank: ranks.typo, distance };
  return null;
}

function search(items, text) {
  const query = toKey(text);
  if (!query) return [];
  const words = text.toLowerCase().split(/\s+/).filter(Boolean);
  const found = [];
  for (const item of items) {
    const rank = getRank(item, query, words);
    if (rank) found.push({ item, ...rank });
  }
  found.sort((left, right) => left.rank - right.rank || left.distance - right.distance || left.item.title.localeCompare(right.item.title));
  return found.slice(0, maxResults).map((result) => result.item);
}

export function SearchBox() {
  const { state, dispatch } = useDocumentation();
  const [text, setText] = useState("");
  const [isOpen, setIsOpen] = useState(false);
  const [active, setActive] = useState(0);
  const box = useRef(null);
  const input = useRef(null);
  const items = useMemo(() => (state.document ? getItems(state.document, state.language) : []), [state.document, state.language]);
  const results = useMemo(() => search(items, text), [items, text]);

  useEffect(() => {
    function focusOnSlash(event) {
      if (event.key !== "/" || event.target.closest("input, textarea")) return;
      event.preventDefault();
      input.current.focus();
    }
    function closeOnOutsideClick(event) {
      if (!box.current.contains(event.target)) setIsOpen(false);
    }
    document.addEventListener("keydown", focusOnSlash);
    document.addEventListener("mousedown", closeOnOutsideClick);
    return () => {
      document.removeEventListener("keydown", focusOnSlash);
      document.removeEventListener("mousedown", closeOnOutsideClick);
    };
  }, []);

  function choose(item) {
    dispatch({ type: actions.entryChosen, entryName: item.target });
    setText("");
    setIsOpen(false);
    input.current.blur();
  }

  function handleKey(event) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((index) => Math.min(index + 1, results.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((index) => Math.max(index - 1, 0));
    } else if (event.key === "Enter" && results[active]) {
      choose(results[active]);
    } else if (event.key === "Escape") {
      setIsOpen(false);
      input.current.blur();
    }
  }

  function handleChange(event) {
    setText(event.target.value);
    setActive(0);
    setIsOpen(true);
  }

  const showList = isOpen && text.trim() !== "";

  return (
    <div className="search" ref={box}>
      <input
        ref={input}
        type="search"
        className="search-input"
        placeholder="Search  /"
        aria-label="Search the documentation"
        value={text}
        onChange={handleChange}
        onFocus={() => setIsOpen(true)}
        onKeyDown={handleKey}
      />
      {showList && (
        <ul className="search-list" role="listbox" aria-label="Search results">
          {results.length === 0 && <li className="search-empty">No match</li>}
          {results.map((item, index) => (
            <li key={item.target} role="option" aria-selected={index === active}>
              <button type="button" onMouseEnter={() => setActive(index)} onClick={() => choose(item)}>
                <span className="search-row">
                  <span className="search-title">{item.title}</span>
                  <span className="search-label">{item.label}</span>
                </span>
                <span className="search-text">{getFirstSentence(item.text)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
