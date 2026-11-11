import { useEffect, useRef, useState } from "react";

import { actions } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";

export function VersionMenu() {
  const { state, dispatch } = useDocumentation();
  const [isOpen, setIsOpen] = useState(false);
  const menu = useRef(null);

  useEffect(() => {
    if (!isOpen) return;
    function closeOnOutsideClick(event) {
      if (!menu.current.contains(event.target)) setIsOpen(false);
    }
    function closeOnEscape(event) {
      if (event.key === "Escape") setIsOpen(false);
    }
    document.addEventListener("mousedown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("mousedown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [isOpen]);

  function chooseVersion(version) {
    dispatch({ type: actions.versionChosen, version });
    setIsOpen(false);
  }

  return (
    <div className="version-menu" ref={menu}>
      <button
        type="button"
        className="version"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label="Version"
        onClick={() => setIsOpen(!isOpen)}
      >
        v{state.version}
      </button>
      {isOpen && (
        <ul className="version-list" role="listbox" aria-label="Versions">
          {state.versions.map((item) => (
            <li key={item.version} role="option" aria-selected={item.version === state.version}>
              <button type="button" onClick={() => chooseVersion(item.version)}>
                v{item.version}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
