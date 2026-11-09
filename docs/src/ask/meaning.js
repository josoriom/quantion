import { getDisplayName } from "../names.js";
import { listOptionFunctions } from "./snippet.js";
import { getWords } from "./values.js";

const meaningPattern = /\b(mean|means|meaning|explain|define|definition)\b|\bwhat\s*('?s|is|are|does|do)\b/i;
const groupOrder = ["Results", "Options", "Inputs", "Enums"];

function findByName(document, name) {
  for (const entry of document.entries) {
    if (entry.name === name) return entry;
  }
  return null;
}

function squash(name) {
  return name.toLowerCase().replace(/[_\s]/g, "");
}

function getItems(entry) {
  return [...(entry.arguments ?? []), ...(entry.fields ?? [])].filter((item) => item.languages?.python?.name && item.meaning);
}

function getPhrases(words) {
  const phrases = [];
  for (let size = 3; size >= 1; size -= 1) {
    for (let start = 0; start + size <= words.length; start += 1) phrases.push(words.slice(start, start + size));
  }
  return phrases;
}

function getQuestionWords(question) {
  return getWords(question.replace(/([a-z])([A-Z])/g, "$1_$2").replace(/_/g, " "));
}

function getSingular(word) {
  return word.length > 3 && word.endsWith("s") ? word.slice(0, -1) : word;
}

export function findNamedEntry(question, document) {
  for (const phrase of getPhrases(getQuestionWords(question))) {
    const joined = phrase.join("");
    for (const name of [joined, getSingular(joined), `${joined}s`]) {
      const entry = document.entries.find((item) => squash(item.name) === name);
      if (entry) return entry;
    }
  }
  return null;
}

function findGroupName(question, document) {
  const words = getQuestionWords(question);
  for (const group of document.groups) {
    if (words.includes(group.name.toLowerCase())) return group;
  }
  return null;
}

function findFieldTerm(question, document) {
  const names = new Set();
  for (const entry of document.entries) {
    for (const item of getItems(entry)) names.add(item.languages.python.name.toLowerCase());
  }
  for (const phrase of getPhrases(getQuestionWords(question))) {
    const joined = phrase.join("_");
    if (names.has(joined)) return joined;
    if (names.has(getSingular(joined))) return getSingular(joined);
  }
  return null;
}

function getRank(entry, contextEntries) {
  const position = contextEntries.indexOf(entry);
  if (position >= 0) return position;
  const group = groupOrder.indexOf(entry.group);
  return contextEntries.length + (group >= 0 ? group : groupOrder.length);
}

function explainField(term, document, contextEntries) {
  const matches = [];
  for (const entry of document.entries) {
    for (const item of getItems(entry)) {
      if (item.languages.python.name.toLowerCase() === term) matches.push({ entry, item, rank: getRank(entry, contextEntries) });
    }
  }
  matches.sort((left, right) => left.rank - right.rank);
  const groups = [];
  for (const match of matches) {
    const group = groups.find((item) => item.meaning === match.item.meaning);
    if (group) group.matches.push(match);
    else groups.push({ meaning: match.item.meaning, matches: [match], isHere: match.rank < contextEntries.length });
  }
  return { type: "field", term, groups };
}

export function isMeaningQuestion(question) {
  return meaningPattern.test(question);
}

export function explainTerm(question, document, contextEntries) {
  if (!isMeaningQuestion(question)) return null;
  const entry = findNamedEntry(question, document);
  const fieldTerm = findFieldTerm(question, document);
  const entryHasField = entry && fieldTerm && !squash(entry.name).includes(squash(fieldTerm)) && getItems(entry).some((item) => item.languages.python.name.toLowerCase() === fieldTerm);
  if (entryHasField) return explainField(fieldTerm, document, [entry, ...contextEntries]);
  if (entry) return { type: "entry", entry };
  if (fieldTerm) return explainField(fieldTerm, document, contextEntries);
  const group = findGroupName(question, document);
  if (group) return { type: "group", group };
  return null;
}

export function getContextEntries(document, names, findOptionsType) {
  const entries = [];
  for (const name of names) {
    const entry = name && findByName(document, name);
    if (!entry) continue;
    for (const item of [entry, findOptionsType(entry, document)]) {
      if (item && !entries.includes(item)) entries.push(item);
    }
  }
  return entries;
}

function getFirstSentence(text) {
  return text.split(/(?<=\.)\s/)[0].replace(/\.$/, "");
}

function lowerFirst(text) {
  return /^[A-Z](?:[a-z]|\s)/.test(text) ? text[0].toLowerCase() + text.slice(1) : text;
}

function withArticle(text) {
  return /^(a|an|the|one|every|each)\b/i.test(text) ? text : `the ${text}`;
}

function code(entry, language) {
  return `\`${getDisplayName(entry, language)}\``;
}

function getArticle(word) {
  return /^[aeiou]/i.test(word) ? "an" : "a";
}

function listFields(entry, language) {
  const names = (entry.fields ?? []).map((field) => field.languages?.[language]?.name ?? field.languages?.python?.name).filter(Boolean);
  if (names.length === 0) return "";
  const shown = names.length > 5 ? [...names.slice(0, 5), "…"] : names;
  return entry.group === "Enums" ? `, one of { ${shown.join(", ")} }` : `, defined by { ${shown.join(", ")} }`;
}

function findMainFunction(entry, document) {
  const name = entry.name.toLowerCase();
  for (const candidate of document.entries) {
    if (candidate.kind !== "function") continue;
    const tokens = candidate.name.split("_");
    if (tokens.some((token) => token === name || token === `${name}s`)) return candidate;
  }
  return null;
}

export function writeExplanation(explanation, document, language) {
  if (explanation.type === "group") {
    const { group } = explanation;
    return { main: `In Quant·ion, ${group.name} is ${lowerFirst(getFirstSentence(group.description))}.`, link: { entryName: group.name, label: group.name } };
  }
  if (explanation.type === "entry") {
    const { entry } = explanation;
    const link = { entryName: entry.name, label: getDisplayName(entry, language) };
    const sentence = getFirstSentence(entry.summary);
    if (entry.kind === "function") return { main: `${code(entry, language)} is a function. ${sentence}.`, link };
    const main = findMainFunction(entry, document);
    const hint = main ? ` To ${main.name.replace(/_/g, " ")}, use ${code(main, language)}.` : "";
    return { main: `In Quant·ion, ${getArticle(entry.name)} ${code(entry, language)} is ${withArticle(lowerFirst(sentence))}${listFields(entry, language)}.${hint}`, link };
  }
  const [first] = explanation.groups;
  const match = first.matches[0];
  const name = match.item.languages[language]?.name ?? explanation.term;
  const functions = listOptionFunctions(match.item.languages.python.name, document);
  if (match.entry.group === "Options" && functions.length > 0) {
    const names = functions.map((entry) => code(entry, language));
    const list = names.length === 1 ? names[0] : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
    return {
      main: `\`${name}\`: ${first.meaning} Use it in ${list}.`,
      link: { entryName: match.entry.name, label: getDisplayName(match.entry, language) },
      option: match.item.languages.python.name,
    };
  }
  return {
    main: `\`${name}\` in ${code(match.entry, language)}: ${first.meaning}`,
    link: { entryName: match.entry.name, label: getDisplayName(match.entry, language) },
  };
}
