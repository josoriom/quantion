import { levenshteinDistance } from "str-utilities";

import { getDisplayName } from "../names.js";
import { findCall, findDefinition, getKind, getOptionsType, toSnakeCase } from "./snippet.js";

const callPattern = /\b([A-Za-z_][A-Za-z0-9_]*)\s*\(/g;
const languages = ["python", "js", "r", "rust"];
export const languageLabels = { python: "Python", js: "JavaScript", r: "R", rust: "Rust" };

export function looksLikeCode(text) {
  if (/<-|::|\bconst\b|\blet\b|\bimport\b|library\(/.test(text)) return true;
  return /\b[A-Za-z_]\w*\s*\([^)]*[,{=(]/.test(text);
}

export function detectLanguage(code, fallback) {
  if (/::|\.\.Default|\blet mut\b|&mut\b/.test(code)) return "rust";
  if (/<-|library\(|\b\w+_options\(/.test(code)) return "r";
  if (/\{\s*["'][\w-]+["']\s*:|^\s*from \w+ import|\bTrue\b|\bFalse\b|\bNone\b/m.test(code)) return "python";
  if (/\b(const|let|await)\b|\{\s*[A-Za-z_]\w*\s*:|\b[a-z]+[A-Z]\w*\s*\(/.test(code)) return "js";
  return fallback;
}

function getAllowedDistance(name) {
  return name.length >= 10 ? 3 : name.length >= 6 ? 2 : 1;
}

function findClosest(name, candidates) {
  let best = null;
  for (const candidate of candidates) {
    const distance = levenshteinDistance(name.toLowerCase(), candidate.toLowerCase());
    if (!best || distance < best.distance) best = { name: candidate, distance };
  }
  if (best && best.distance <= getAllowedDistance(name)) return best.name;
  return null;
}

function getFunctionNames(document, language) {
  const names = new Map();
  for (const entry of document.entries) {
    if (entry.kind === "function") names.set(getDisplayName(entry, language), entry);
  }
  return names;
}

function getHelperNames(document) {
  const names = new Set(["print", "len", "range", "library", "head", "data", "frame", "list", "vec", "Some", "Ok", "drop", "main", "log", "c", "cat", "sprintf"]);
  for (const entry of document.entries) {
    if (entry.kind !== "function") names.add(entry.name);
    if (entry.group === "Options") names.add(toSnakeCase(entry.name));
  }
  return names;
}

function getOptionFields(entry, document) {
  const fields = [];
  for (const field of getOptionsType(entry, document)?.fields ?? []) {
    fields.push({ field, owner: entry });
    const kind = getKind(field.languages.rust?.type ?? "", document);
    if (kind.kind !== "options") continue;
    for (const inner of kind.entry.fields ?? []) fields.push({ field: inner, owner: entry });
  }
  return fields;
}

function findOptionElsewhere(name, document, language, entry) {
  for (const candidate of document.entries) {
    if (candidate.kind !== "function" || candidate === entry) continue;
    for (const { field } of getOptionFields(candidate, document)) {
      if (field.languages[language]?.name === name) return candidate;
    }
  }
  return null;
}

function getOptionsText(code, call, language) {
  const last = call.pieces[call.pieces.length - 1];
  if (!last) return null;
  let text = code.slice(last.start, last.end).trim().replace(/^options\s*=\s*/, "");
  if (/^[A-Za-z_]\w*$/.test(text)) {
    const definition = findDefinition(code, text);
    if (!definition) return null;
    text = code.slice(definition.start, definition.end);
  }
  if (language === "python" && !text.startsWith("{")) return null;
  if (language === "js" && !text.startsWith("{")) return null;
  if (language === "r" && !/_options\(/.test(text)) return null;
  if (language === "rust" && !/Options\s*\{/.test(text)) return null;
  return text;
}

function findKeys(text, language) {
  const keys = [];
  if (language === "python") {
    for (const match of text.matchAll(/["']([\w-]+)["']\s*:/g)) keys.push({ name: match[1], index: match.index });
    return keys;
  }
  const bare = text.replace(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g, (literal) => " ".repeat(literal.length));
  const pattern = language === "r" ? /([A-Za-z_][\w.]*)\s*=(?!=)/g : /([A-Za-z_]\w*)\s*:(?!:)/g;
  for (const match of bare.matchAll(pattern)) keys.push({ name: match[1], index: match.index });
  return keys;
}

function readValueText(text, key) {
  const after = text.slice(key.index + key.name.length + (text[key.index] === '"' || text[key.index] === "'" ? 2 : 0));
  const match = /^\s*["']?\s*[:=]\s*(["'][^"']*["']|[\w.+-]+)/.exec(after);
  return match ? match[1] : null;
}

function checkValue(field, valueText, document, language) {
  if (!valueText) return null;
  const kind = getKind(field.languages.rust?.type ?? "", document);
  const quoted = /^["'](.*)["']$/.exec(valueText);
  if ((kind.kind === "number" || kind.kind === "integer") && quoted && !Number.isNaN(Number(quoted[1]))) {
    return { from: valueText, to: quoted[1], reason: `\`${field.languages[language].name}\` takes a number, not text.` };
  }
  if (kind.kind === "boolean") {
    const right = { python: { true: "True", false: "False" }, r: { true: "TRUE", false: "FALSE" } }[language] ?? { true: "true", false: "false" };
    const lower = valueText.toLowerCase();
    if (right[lower] && right[lower] !== valueText) return { from: valueText, to: right[lower], reason: `${languageLabels[language]} writes booleans as ${right.true}/${right.false}.` };
  }
  return null;
}

function reviewOptions(code, entry, language, document, fixes, notes) {
  const call = findCall(code, getDisplayName(entry, language));
  if (!call) return;
  const text = getOptionsText(code, call, language);
  if (!text) return;
  const fields = getOptionFields(entry, document);
  const valid = new Map(fields.map(({ field }) => [field.languages[language]?.name, field]));
  const functionName = `\`${getDisplayName(entry, language)}\``;
  for (const key of findKeys(text, language)) {
    const field = valid.get(key.name);
    if (field) {
      const valueFix = checkValue(field, readValueText(text, key), document, language);
      if (valueFix && !fixes.some((fix) => fix.from === valueFix.from)) fixes.push({ ...valueFix, context: key.name });
      continue;
    }
    const otherForm = fields.find(({ field: item }) => languages.some((other) => item.languages[other]?.name === key.name));
    if (otherForm) {
      const right = otherForm.field.languages[language].name;
      fixes.push({ from: key.name, to: right, reason: `${languageLabels[language]} names this option \`${right}\`.` });
      continue;
    }
    const closest = findClosest(key.name, [...valid.keys()].filter(Boolean));
    if (closest) {
      fixes.push({ from: key.name, to: closest, reason: `\`${key.name}\` is not an option of ${functionName}; did you mean \`${closest}\`?` });
      continue;
    }
    const owner = findOptionElsewhere(key.name, document, language, entry);
    if (owner) notes.push(`\`${key.name}\` is an option of \`${getDisplayName(owner, language)}\`, not of ${functionName}.`);
    else notes.push(`\`${key.name}\` is not an option of ${functionName}. Its options: ${[...valid.keys()].slice(0, 6).map((name) => `\`${name}\``).join(", ")}…`);
  }
}

function applyFixes(code, fixes) {
  let fixed = code;
  for (const fix of fixes) {
    const isName = /^[A-Za-z_]\w*$/.test(fix.from);
    if (isName) fixed = fixed.replace(new RegExp(`(?<![\\w.])${fix.from}(?!\\w)`, "g"), fix.to);
    else if (fix.context) fixed = fixed.replace(new RegExp(`(${fix.context}["']?\\s*[:=]\\s*)${fix.from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`), `$1${fix.to}`);
  }
  return fixed;
}

export function reviewCode(code, document, fallbackLanguage) {
  const language = detectLanguage(code, fallbackLanguage);
  const functionNames = getFunctionNames(document, language);
  const helperNames = getHelperNames(document);
  const fixes = [];
  const notes = [];
  const found = [];

  for (const match of code.matchAll(callPattern)) {
    const name = match[1];
    if (functionNames.has(name)) {
      const entry = functionNames.get(name);
      if (!found.includes(entry)) found.push(entry);
      continue;
    }
    const otherLanguage = languages.find((other) => getFunctionNames(document, other).has(name));
    if (otherLanguage && !helperNames.has(name)) {
      const entry = getFunctionNames(document, otherLanguage).get(name);
      const right = getDisplayName(entry, language);
      if (!fixes.some((fix) => fix.from === name)) fixes.push({ from: name, to: right, reason: `${languageLabels[language]} names this function \`${right}\`.` });
      if (!found.includes(entry)) found.push(entry);
      continue;
    }
    if (helperNames.has(name)) continue;
    const closest = findClosest(name, [...functionNames.keys()]);
    if (closest && !fixes.some((fix) => fix.from === name)) {
      fixes.push({ from: name, to: closest, reason: `There is no function \`${name}\`; did you mean \`${closest}\`?` });
      const entry = functionNames.get(closest);
      if (!found.includes(entry)) found.push(entry);
    }
  }

  const named = applyFixes(code, fixes.filter((fix) => /^[A-Za-z_]\w*$/.test(fix.from)));
  for (const entry of found) reviewOptions(named, entry, language, document, fixes, notes);
  return { language, entries: found, fixes, notes, code: applyFixes(code, fixes) };
}
