import { levenshteinDistance } from "str-utilities";

import { getDisplayName } from "../names.js";
import { guideMenu } from "./guide.js";
import { reviewCode } from "./review.js";
import { buildSnippet, listValueKeys, validateValues } from "./snippet.js";

export const commandList = [
  { name: "help", usage: "/help", meaning: "Lists the commands and 3 example questions." },
  { name: "guide", usage: "/guide", meaning: "Pick what you want to do, step by step." },
  { name: "docs", usage: "/docs <name>", meaning: "Opens that docs page; alone, opens the overview." },
  { name: "example", usage: "/example <function> [key=value …]", meaning: "The template with your values filled in." },
  { name: "fix", usage: "/fix <code>", meaning: "Checks pasted code: names, options and values." },
];

export const exampleQuestions = ["How do I find peaks above intensity 1000?", "What is a target?", "I have mz and rt, what do I use?"];

export function isCommand(text) {
  return /^\s*\//.test(text);
}

function squash(name) {
  return name.toLowerCase().replace(/[_\s-]/g, "");
}

function getAllowedDistance(name) {
  return name.length >= 10 ? 3 : name.length >= 6 ? 2 : name.length >= 4 ? 1 : 0;
}

export function resolveName(name, document) {
  const wanted = squash(name);
  const candidates = [];
  for (const entry of document.entries) candidates.push({ key: squash(entry.name), target: { kind: "entry", entryName: entry.name } });
  for (const group of document.groups ?? []) candidates.push({ key: squash(group.name), target: { kind: "group", entryName: group.name } });
  candidates.push({ key: "overview", target: { kind: "page", entryName: "overview" } });
  let best = null;
  for (const candidate of candidates) {
    const distance = levenshteinDistance(wanted, candidate.key);
    if (!best || distance < best.distance) best = { ...candidate, distance };
  }
  if (best && best.distance <= getAllowedDistance(wanted)) return best.target;
  return null;
}

function resolveFunction(name, document) {
  const target = resolveName(name, document);
  if (!target || target.kind !== "entry") return null;
  const entry = document.entries.find((item) => item.name === target.entryName);
  return entry?.kind === "function" ? entry : null;
}

function readValue(text) {
  const clean = text.trim();
  if (/^-?\d+(\.\d+)?$/.test(clean)) return Number(clean);
  if (/^(true|yes|on)$/i.test(clean)) return true;
  if (/^(false|no|off)$/i.test(clean)) return false;
  return clean.replace(/^["']|["']$/g, "");
}

function parsePairs(text) {
  const values = {};
  const bad = [];
  for (const piece of text.split(/\s+/).filter(Boolean)) {
    const match = /^([A-Za-z_][\w]*)\s*=\s*(.+)$/.exec(piece);
    if (!match) bad.push(piece);
    else values[match[1]] = readValue(match[2]);
  }
  return { values, bad };
}

function listKeys(entry, document) {
  return listValueKeys(entry, document).map((key) => key.item.languages.python.name);
}

export function runCommand(text, document, language) {
  const match = /^\s*\/(\w*)\s*([\s\S]*)$/.exec(text);
  if (!match) return null;
  const [, name, rest] = match;
  const argument = rest.trim();
  if (name === "help" || name === "") return { command: "help", commands: commandList, examples: exampleQuestions };
  if (name === "guide") return { command: "guide", items: guideMenu };
  if (name === "docs") {
    if (!argument) return { command: "docs", target: { kind: "page", entryName: "overview" }, label: "Overview" };
    const target = resolveName(argument, document);
    if (!target) return { command: "docs", target: null, failure: `No docs page is named \`${argument}\`.` };
    const entry = document.entries.find((item) => item.name === target.entryName);
    return { command: "docs", target, label: entry ? getDisplayName(entry, language) : target.entryName };
  }
  if (name === "example") {
    const [functionName, ...pairs] = argument.split(/\s+/).filter(Boolean);
    if (!functionName) return { command: "example", failure: "Give a function name: `/example find_peaks min_intensity=1000`." };
    const entry = resolveFunction(functionName, document);
    if (!entry) return { command: "example", failure: `No function is named \`${functionName}\`.` };
    const { values, bad } = parsePairs(pairs.join(" "));
    if (bad.length > 0) return { command: "example", entry, failure: `Write each value as key=value, not \`${bad[0]}\`.` };
    const checked = validateValues({ function: entry.name, ...values }, document);
    if (!checked.valid) {
      const keys = listKeys(entry, document);
      const hint = keys.length > 0 ? ` Values of \`${getDisplayName(entry, language)}\`: ${keys.map((key) => `\`${key}\``).join(", ")}.` : "";
      return { command: "example", entry, failure: `${checked.reason[0].toUpperCase()}${checked.reason.slice(1)}.${hint}` };
    }
    const normalized = { function: entry.name };
    for (const item of checked.argumentValues) normalized[item.argument.languages.python.name] = item.value;
    for (const item of checked.optionValues) normalized[item.path[item.path.length - 1].languages.python.name] = item.value;
    return { command: "example", entry, values: normalized, snippet: buildSnippet(normalized, document, language) };
  }
  if (name === "fix") {
    if (!argument) return { command: "fix", failure: "Paste the code after `/fix`." };
    const review = reviewCode(argument, document, language);
    if (review.entries.length === 0) return { command: "fix", failure: "I found no Quant·ion function in that code." };
    return { command: "fix", review };
  }
  return { command: "unknown", name, commands: commandList, failure: `There is no command \`/${name}\`.` };
}
