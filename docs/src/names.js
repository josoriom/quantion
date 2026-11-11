export function getDisplayName(entry, language) {
  if (entry.kind !== "function" || language !== "js") return entry.name;
  return entry.name.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

export function getFirstSentence(text) {
  return text.split(/(?<=\.)\s/)[0];
}
