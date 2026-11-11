import { getDisplayName } from "../names.js";
import { listOptionFunctions } from "./snippet.js";

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
    const address = match.item.languages[language]?.default?.match(/^"(https?:\/\/[^"]+)"$/)?.[1];
    const fallback = address ? ` By default it is ${address}.` : "";
    return {
      main: `\`${name}\`: ${first.meaning}${fallback} Use it in ${list}.`,
      link: { entryName: match.entry.name, label: getDisplayName(match.entry, language) },
      option: match.item.languages.python.name,
    };
  }
  return {
    main: `\`${name}\` in ${code(match.entry, language)}: ${first.meaning}`,
    link: { entryName: match.entry.name, label: getDisplayName(match.entry, language) },
  };
}
