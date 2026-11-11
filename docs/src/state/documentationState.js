import { produce } from "immer";

export const initialState = {
  versions: [],
  version: null,
  document: null,
  language: "python",
  entryName: "overview",
  status: "loading",
};

export const actions = {
  versionsLoaded: "versionsLoaded",
  versionChosen: "versionChosen",
  documentLoaded: "documentLoaded",
  documentFailed: "documentFailed",
  languageChosen: "languageChosen",
  entryChosen: "entryChosen",
};

export const documentationReducer = produce((state, action) => {
  switch (action.type) {
    case actions.versionsLoaded:
      state.versions = action.versions;
      state.version = action.latest;
      break;
    case actions.versionChosen:
      state.version = action.version;
      state.status = "loading";
      break;
    case actions.documentLoaded:
      state.document = action.document;
      state.status = "ready";
      if (!isKnownPage(action.document, state.entryName)) state.entryName = "overview";
      if (!findLanguage(action.document, state.language)) state.language = initialState.language;
      break;
    case actions.documentFailed:
      state.status = "failed";
      break;
    case actions.languageChosen:
      state.language = action.language;
      break;
    case actions.entryChosen:
      state.entryName = action.entryName;
      break;
  }
});

function findByKey(items, key, value) {
  for (const item of items) {
    if (item[key] === value) return item;
  }
  return null;
}

export function findApiFile(versions, version) {
  const item = findByKey(versions, "version", version);
  if (!item) return null;
  return item.api;
}

export function findEntry(document, entryName) {
  if (!document) return null;
  return findByKey(document.entries, "name", entryName);
}

export function findLanguage(document, languageKey) {
  return findByKey(document.languages, "key", languageKey);
}

export function findGroup(document, groupName) {
  if (!document) return null;
  return findByKey(document.groups, "name", groupName);
}

export function getGroupEntries(document, groupName) {
  const entries = [];
  for (const entry of document.entries) {
    if (entry.group === groupName) entries.push(entry);
  }
  return entries;
}

export const pages = { overview: "overview", versions: "versions" };

function isKnownPage(document, name) {
  if (Object.values(pages).includes(name)) return true;
  if (findEntry(document, name)) return true;
  if (findGroup(document, name)) return true;
  return false;
}
