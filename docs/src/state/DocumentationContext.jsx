import { createContext, useContext, useEffect, useReducer } from "react";

import { actions, documentationReducer, findApiFile, initialState } from "./documentationState";

const DocumentationContext = createContext(null);

async function readJson(path) {
  const response = await fetch(path, { cache: "no-cache" });
  if (!response.ok) throw new Error(`${path} answered ${response.status}`);
  return response.json();
}

function readSavedLanguage() {
  try {
    return localStorage.getItem("quantion-language");
  } catch {
    return null;
  }
}

function saveLanguage(language) {
  try {
    localStorage.setItem("quantion-language", language);
  } catch {
    return;
  }
}

function readEntryFromAddress() {
  return decodeURIComponent(location.hash.slice(1)) || "overview";
}

function readLanguageFromAddress() {
  return new URLSearchParams(location.search).get("language");
}

function getStartState() {
  return {
    ...initialState,
    language: readLanguageFromAddress() ?? readSavedLanguage() ?? initialState.language,
    entryName: readEntryFromAddress(),
  };
}

export function DocumentationProvider({ children }) {
  const [state, dispatch] = useReducer(documentationReducer, undefined, getStartState);

  useEffect(() => {
    readJson("./api/versions.json")
      .then((list) => dispatch({ type: actions.versionsLoaded, versions: list.versions, latest: list.latest }))
      .catch(() => dispatch({ type: actions.documentFailed }));
  }, []);

  const apiFile = findApiFile(state.versions, state.version);

  useEffect(() => {
    if (!apiFile) return;
    readJson(`./api/${apiFile}.json`)
      .then((document) => dispatch({ type: actions.documentLoaded, document }))
      .catch(() => dispatch({ type: actions.documentFailed }));
  }, [apiFile]);

  useEffect(() => {
    saveLanguage(state.language);
  }, [state.language]);

  useEffect(() => {
    history.replaceState(null, "", `?language=${state.language}#${encodeURIComponent(state.entryName)}`);
  }, [state.entryName, state.language]);

  return <DocumentationContext.Provider value={{ state, dispatch }}>{children}</DocumentationContext.Provider>;
}

export function useDocumentation() {
  return useContext(DocumentationContext);
}
