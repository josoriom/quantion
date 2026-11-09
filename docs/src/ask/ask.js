import { getDisplayName } from "../names.js";
import { isCommand, runCommand } from "./commands.js";
import { findVariant, getFunctionFollowUps, guideItems, hasVariants, listVariants, matchGuideKeywords, pickGuideItem, starters } from "./guide.js";
import { limits, models } from "./models.js";
import { listOptionFunctions, listValueKeys } from "./snippet.js";
import { explainTerm, findNamedEntry, getContextEntries, isMeaningQuestion } from "./meaning.js";
import { looksLikeCode, reviewCode } from "./review.js";
import { applySynonyms, createTypoFixer, hasDomainWord } from "./words.js";
import { createValueReader, getWords } from "./values.js";

const genericTokens = new Set(["get", "options"]);
const keyTokensIgnored = new Set(["min", "max"]);
const rangeKeys = new Set(["from", "to", "start", "end"]);
const tokenCounts = new WeakMap();
const exactTokens = new WeakMap();
const nameTextPosition = 1;

function findByName(document, name) {
  for (const entry of document.entries) {
    if (entry.name === name) return entry;
  }
  return null;
}

function getRustType(item) {
  return item.languages.rust?.type ?? "";
}

export function findOptionsType(entry, document) {
  for (const argument of entry.arguments ?? []) {
    const type = findByName(document, getRustType(argument));
    if (type && type.group === "Options") return type;
  }
  return null;
}

export function findFunctionFor(entry, document) {
  if (entry.kind === "function") return entry;
  for (const candidate of document.entries) {
    if (candidate.kind !== "function") continue;
    if (findOptionsType(candidate, document) === entry) return candidate;
  }
  const namePattern = new RegExp(`\\b${entry.name}\\b`);
  for (const candidate of document.entries) {
    if (candidate.kind !== "function") continue;
    if (namePattern.test(candidate.languages.rust?.signature ?? "")) return candidate;
    const optionsType = findOptionsType(candidate, document);
    for (const field of optionsType?.fields ?? []) {
      if (namePattern.test(getRustType(field))) return candidate;
    }
  }
  return entry;
}

function describeItems(items, language) {
  const parts = [];
  for (const item of items ?? []) parts.push(`${item.languages[language].name}: ${item.meaning}`);
  return parts.join("; ");
}

export function describeEntry(entry, document) {
  const parts = [`${entry.name}.`, `${entry.group}.`, entry.summary];
  if (entry.arguments) parts.push(`Arguments: ${describeItems(entry.arguments, "python")}.`);
  if (entry.fields) parts.push(`Fields: ${describeItems(entry.fields, "python")}.`);
  const optionsType = findOptionsType(entry, document);
  if (optionsType) parts.push(`Options: ${describeItems(optionsType.fields, "python")}.`);
  const example = entry.languages.python?.example;
  if (example) parts.push(`Example: ${example}`);
  return parts.join(" ");
}

function toWords(name) {
  return name.replace(/_/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
}

function splitSentences(text) {
  return text.split(/(?<=\.)\s+/).filter((sentence) => sentence.length > 3);
}

function findGroup(document, name) {
  return (document.groups ?? []).find((group) => group.name === name) ?? null;
}

export function describeEntryTexts(entry, document) {
  const texts = [describeEntry(entry, document), toWords(entry.name), ...splitSentences(entry.summary)];
  if (entry.kind !== "function") return texts;
  const words = toWords(entry.name);
  texts.push(`How do I ${splitSentences(entry.summary)[0].replace(/\.$/, "").replace(/^\w/, (letter) => letter.toLowerCase())}?`);
  for (const argument of entry.arguments ?? []) {
    const name = argument.languages.python?.name;
    if (name && argument.meaning) texts.push(`${words} ${toWords(name)}: ${argument.meaning}`);
  }
  for (const field of findOptionsType(entry, document)?.fields ?? []) {
    texts.push(`${words} ${toWords(field.languages.python.name)}: ${field.meaning}`);
  }
  const group = findGroup(document, entry.group);
  if (group) texts.push(`${group.name}: ${splitSentences(group.description)[0]}`);
  return texts;
}

export function encodeVectors(vectors) {
  const bytes = new Int8Array(vectors.length * vectors[0].length);
  vectors.forEach((vector, row) => vector.forEach((value, column) => (bytes[row * vector.length + column] = Math.round(value * 127))));
  let text = "";
  for (const byte of new Uint8Array(bytes.buffer)) text += String.fromCharCode(byte);
  return btoa(text);
}

function decodeVectors(text, dimension) {
  const raw = atob(text);
  const bytes = new Int8Array(raw.length);
  for (let position = 0; position < raw.length; position += 1) bytes[position] = (raw.charCodeAt(position) << 24) >> 24;
  const vectors = [];
  for (let start = 0; start < bytes.length; start += dimension) vectors.push(Float32Array.from(bytes.subarray(start, start + dimension), (value) => value / 127));
  return vectors;
}

export function trimAnswer(text) {
  const clean = text.replace(/\s+/g, " ").trim();
  if (clean.length <= limits.answer) return clean;
  const head = clean.slice(0, limits.answer);
  const sentenceEnd = head.lastIndexOf(". ");
  if (sentenceEnd > limits.answer / 3) return head.slice(0, sentenceEnd + 1);
  return `${head.slice(0, head.lastIndexOf(" "))}…`;
}

const decodedIndexes = new WeakMap();

function getDecodedEntries(index) {
  if (!decodedIndexes.has(index)) {
    decodedIndexes.set(index, index.entries.map((item) => ({ name: item.name, vectors: decodeVectors(item.vectors, index.dimension) })));
  }
  return decodedIndexes.get(index);
}

function getDotProduct(left, right) {
  let score = 0;
  for (let position = 0; position < left.length; position += 1) score += left[position] * right[position];
  return score;
}

function getBestScore(vector, vectors, skipPosition = -1) {
  let best = -Infinity;
  vectors.forEach((stored, position) => {
    if (position === skipPosition) return;
    best = Math.max(best, getDotProduct(vector, stored));
  });
  return best;
}

const decodedAnchors = new WeakMap();

export function getOffTopicScore(vector, index) {
  if (!index.anchors) return -Infinity;
  if (!decodedAnchors.has(index)) decodedAnchors.set(index, decodeVectors(index.anchors, index.dimension));
  return getBestScore(vector, decodedAnchors.get(index));
}

const decodedGuides = new WeakMap();

export function getGuideScores(vector, index) {
  if (!index.guide) return new Map();
  if (!decodedGuides.has(index)) decodedGuides.set(index, index.guide.map((item) => ({ id: item.id, vectors: decodeVectors(item.vectors, index.dimension) })));
  const scores = new Map();
  for (const item of decodedGuides.get(index)) scores.set(item.id, getBestScore(vector, item.vectors));
  return scores;
}

export function rankEntries(vector, index, options = {}) {
  const ranked = [];
  for (const item of getDecodedEntries(index)) {
    ranked.push({ name: item.name, score: getBestScore(vector, item.vectors, options.descriptive ? nameTextPosition : -1) });
  }
  ranked.sort((left, right) => right.score - left.score);
  return ranked;
}

export function rankWithVectors(vectors, index, options = {}) {
  const best = new Map();
  for (const vector of vectors) {
    for (const item of rankEntries(vector, index, options)) {
      if (!best.has(item.name) || best.get(item.name) < item.score) best.set(item.name, item.score);
    }
  }
  return [...best.entries()].map(([name, score]) => ({ name, score })).sort((left, right) => right.score - left.score);
}

function sharesStart(left, right) {
  const length = Math.min(5, left.length, right.length);
  return length >= 3 && left.slice(0, length) === right.slice(0, length);
}

function getNameTokens(entry) {
  if (entry?.kind === "function") return entry.name.split("_");
  if (entry?.group !== "Options") return [];
  return entry.name.replace(/Options$/, "").split(/(?=[A-Z])/).map((token) => token.toLowerCase());
}

function countFunctionsWithToken(document, token) {
  if (!tokenCounts.has(document)) tokenCounts.set(document, new Map());
  const counts = tokenCounts.get(document);
  if (!counts.has(token)) {
    let count = 0;
    for (const entry of document.entries) {
      if (entry.kind === "function" && entry.name.split("_").some((other) => sharesStart(other, token))) count += 1;
    }
    counts.set(token, count);
  }
  return counts.get(token);
}

function getExactTokens(document) {
  if (!exactTokens.has(document)) {
    const tokens = new Set();
    for (const entry of document.entries) {
      if (entry.kind === "function") for (const token of entry.name.split("_")) tokens.add(token);
    }
    exactTokens.set(document, tokens);
  }
  return exactTokens.get(document);
}

function matchesToken(word, token, document) {
  if (word === token) return true;
  if (word.length > token.length && getExactTokens(document).has(word)) return false;
  return sharesStart(word, token);
}

function hasNamePhrase(tokens, words, document) {
  if (tokens.length < 2) return false;
  for (let start = 0; start + tokens.length <= words.length; start += 1) {
    if (tokens.every((token, offset) => matchesToken(words[start + offset], token, document))) return true;
  }
  return false;
}

export function addNameBonus(ranked, question, document) {
  const words = getWords(question);
  const scored = [];
  for (const item of ranked) {
    let weight = 0;
    const entry = findByName(document, item.name);
    const tokens = getNameTokens(entry);
    for (const token of tokens) {
      if (genericTokens.has(token) || !words.some((word) => matchesToken(word, token, document))) continue;
      weight += countFunctionsWithToken(document, token) === 1 ? 2 : 1;
    }
    if (entry?.kind === "function" && hasNamePhrase(tokens, words, document)) weight += 1;
    if (entry?.kind === "function" && words.includes(entry.group.toLowerCase())) weight += 1;
    scored.push({ ...item, score: item.score + Math.min(weight, limits.nameWeightCap) * limits.nameBonus });
  }
  return scored.sort((left, right) => right.score - left.score);
}

function namesKey(key, words) {
  const name = key.item.languages.python.name;
  if (rangeKeys.has(name)) return false;
  const tokens = name.split("_").filter((token) => !keyTokensIgnored.has(token));
  return tokens.length > 0 && tokens.every((token) => words.includes(token));
}

function takesNumber(entry, document, words) {
  for (const owner of [entry, ...getTemplateFunctions(entry, document)]) {
    if (listValueKeys(owner, document).some((key) => (key.kind.kind === "number" || key.kind.kind === "integer") && namesKey(key, words))) return true;
  }
  return false;
}

export function addKeyBonus(ranked, question, document) {
  if (!/\d/.test(question)) return ranked;
  const words = getWords(question);
  const scored = ranked.map((item) => {
    const entry = findByName(document, item.name);
    if (entry?.kind !== "function") return item;
    return takesNumber(entry, document, words) ? { ...item, score: item.score + limits.keyBonus } : item;
  });
  return scored.sort((left, right) => right.score - left.score);
}

export function pickEntries(ranked, document) {
  const functions = [];
  const others = [];
  for (const item of ranked) {
    const entry = findFunctionFor(findByName(document, item.name), document);
    const list = entry.kind === "function" ? functions : others;
    if (!list.includes(entry)) list.push(entry);
  }
  return [...functions, ...others].slice(0, limits.entries);
}

function getFunctionScores(ranked, document) {
  const scores = new Map();
  for (const item of ranked) {
    const entry = findFunctionFor(findByName(document, item.name), document);
    if (!scores.has(entry.name)) scores.set(entry.name, item.score);
  }
  return scores;
}

export function findExampleEntry(entries, language) {
  for (const entry of entries) {
    if (entry.languages[language]?.example) return entry;
  }
  return null;
}

function useLocalRuntime(wasm, runtimeFiles) {
  if (!wasm || !runtimeFiles) return;
  const paths = typeof wasm.wasmPaths === "object" && wasm.wasmPaths ? wasm.wasmPaths : { mjs: "ort-wasm-simd-threaded.asyncify.mjs", wasm: "ort-wasm-simd-threaded.asyncify.wasm" };
  const local = {};
  for (const [kind, address] of Object.entries(paths)) {
    const name = String(address).split("/").pop();
    if (!runtimeFiles[name]) throw new Error(`No local ONNX runtime file ${name}`);
    local[kind] = runtimeFiles[name];
  }
  wasm.wasmPaths = local;
}

async function loadTransformers(options) {
  if (!options.modelFolder) throw new Error("createEmbedder needs a modelFolder");
  const transformers = await import("@huggingface/transformers");
  transformers.env.allowLocalModels = true;
  transformers.env.allowRemoteModels = false;
  transformers.env.localModelPath = options.modelFolder;
  useLocalRuntime(transformers.env.backends?.onnx?.wasm, options.runtimeFiles);
  return transformers;
}

export async function createEmbedder(options = {}) {
  const transformers = await loadTransformers(options);
  const extractor = await transformers.pipeline("feature-extraction", models.embedder.name, {
    dtype: models.embedder.dtype,
    device: options.device,
    progress_callback: options.onProgress,
  });
  return async (text) => {
    const output = await extractor(text, { pooling: "mean", normalize: true });
    return Array.from(output.data);
  };
}

export function blendVectors(current, earlier, weight) {
  const blended = current.map((value, position) => value + weight * earlier[position]);
  const length = Math.hypot(...blended);
  return blended.map((value) => value / length);
}

function formatAnswerValue(value) {
  if (typeof value === "string") return `"${value}"`;
  return String(value);
}

function findValueItem(entry, document, name) {
  for (const key of listValueKeys(entry, document)) {
    if (key.item.languages.python.name === name) return key.item;
  }
  return null;
}

function describeValues(entry, document, language, values) {
  const parts = [];
  for (const [name, value] of Object.entries(values)) {
    if (name === "function") continue;
    const item = findValueItem(entry, document, name);
    parts.push(`\`${item.languages[language].name}\` = ${formatAnswerValue(value)}`);
  }
  return parts.join(" and ");
}

export function listUnderstood(result, document, language) {
  const entry = result.entries[0];
  const chips = [{ kind: "function", entryName: entry.name, label: getDisplayName(entry, language) }];
  for (const [name, value] of Object.entries(result.values ?? {})) {
    if (name === "function") continue;
    const item = findValueItem(entry, document, name);
    chips.push({ kind: "value", owner: entry.name, name, label: `${item.languages[language].name} = ${formatAnswerValue(value)}`, value });
  }
  for (const [ownerName, values] of Object.entries(result.templateValues ?? {})) {
    const owner = findByName(document, ownerName);
    for (const [name, value] of Object.entries(values)) {
      const item = findValueItem(owner, document, name);
      chips.push({ kind: "value", owner: ownerName, name, label: `${item.languages[language].name} = ${formatAnswerValue(value)} (${getDisplayName(owner, language)})`, value });
    }
  }
  return chips;
}

export function writeAnswer(result, document, language) {
  const entry = result.entries[0];
  const functionName = `\`${getDisplayName(entry, language)}\``;
  const own = describeValues(entry, document, language, result.values);
  const helpers = [];
  for (const [name, values] of Object.entries(result.templateValues ?? {})) {
    const helper = document.entries.find((item) => item.name === name);
    helpers.push(`\`${getDisplayName(helper, language)}\` with ${describeValues(helper, document, language, values)}`);
  }
  let text;
  if (result.variant?.refused) text = result.variant.refused;
  else if (own) text = `Use ${functionName} with ${own}.`;
  else if (result.variant) text = `Use ${functionName} with the ${result.variant.label.toLowerCase().replace(/^from (a |an )?/, "")}: ${splitSentences(entry.summary)[0]}`;
  else if (helpers.length === 0 && result.mentioned) text = `Use ${functionName} and set its option \`${result.mentioned.languages[language].name}\`: ${result.mentioned.meaning}`;
  else text = `Use ${functionName}: ${entry.summary}`;
  if (helpers.length > 0) text += ` In the template, use ${helpers.join(" and ")}.`;
  else if (result.helperMention) {
    const { owner, item } = result.helperMention;
    text += ` The \`${item.languages[language].name}\` goes in \`${getDisplayName(owner, language)}\`: ${item.meaning}`;
  }
  return trimAnswer(text);
}

const referencePattern = /\b(it|its|they|them|those|these|that|this|one|ones|same)\b|^\s*(and|also|but|then|now|what about|or)\b/i;
const continuationPattern = /^\s*(or|and|what about|how about|also|instead)\b/i;
const questionStartPattern = /^\s*(how|what|which|where|why|when|can|could|is|are|do|does|should|i|my|the)\b/i;

export function refersBack(question) {
  return referencePattern.test(question);
}

export function continuesTopic(question) {
  if (continuationPattern.test(question)) return true;
  const words = question.trim().split(/\s+/);
  return words.length <= 6 && !questionStartPattern.test(question) && !/\?$/.test(question.trim());
}

function isAboutPage(explanation, page) {
  if (!page) return false;
  if (explanation.type === "entry") return explanation.entry.name === page;
  if (explanation.type === "field") return explanation.groups[0].isHere;
  return false;
}

const placementPattern = /\bwhere\b.*\b(place|put|set|pass|go|goes|give|add|write|enter|specify)\b|\bwhich (arguments?|options?|parameters?|fields?)\b/i;

function findPlacement(question, entry, document) {
  if (!entry || !placementPattern.test(question)) return null;
  const words = getWords(question);
  const owners = [entry, ...getTemplateFunctions(entry, document)];
  for (const owner of owners) {
    for (const key of listValueKeys(owner, document)) {
      const tokens = key.item.languages.python.name.split("_").filter((token) => !["min", "max"].includes(token));
      if (tokens.length > 0 && tokens.every((token) => words.includes(token))) return { entry, owner, item: key.item };
    }
  }
  return null;
}

export function writePlacement(placement, language) {
  const { entry, owner, item } = placement;
  const text = `Put it in \`${item.languages[language].name}\` of \`${getDisplayName(owner, language)}\`: ${item.meaning}`;
  const example = entry.languages[language]?.example ?? "";
  const line = example.split("\n").find((row) => row.includes(`${getDisplayName(owner, language)}(`));
  return { text, line: line?.trim() ?? null, entry };
}

export function getTemplateFunctions(entry, document) {
  const example = entry?.languages.python?.example ?? "";
  const found = [];
  for (const candidate of document.entries) {
    if (candidate.kind !== "function" || candidate === entry || candidate.name === "dispose") continue;
    if (new RegExp(`\\b${candidate.name}\\(`).test(example)) found.push(candidate);
  }
  return found;
}

const numberValuePattern = /\d+(\.\d+)?/;

function namesFunction(entry, text) {
  if (entry?.kind !== "function") return false;
  if (isCodeForm(entry, text)) return true;
  const squashed = entry.name.replace(/_/g, "");
  const words = getWords(text);
  for (let start = 0; start < words.length; start += 1) {
    if (words.slice(start, start + entry.name.split("_").length).join("") === squashed) return true;
  }
  return false;
}

function isCodeForm(entry, text) {
  if (!entry.name.includes("_")) return new RegExp(`\\b${entry.name}\\(|\\.${entry.name}\\b`, "i").test(text);
  const snake = entry.name.replace(/s$/, "");
  const camel = getDisplayName(entry, "js").replace(/s$/, "");
  return new RegExp(`(^|[^\\w.])(${snake}|${camel})s?(\\(|\\b)`, "i").test(text);
}

const plainWords = new Set(["how", "do", "does", "i", "can", "could", "should", "to", "use", "using", "it", "this", "that", "them", "one", "work", "works", "call", "run", "show", "me", "an", "a", "the", "example", "please", "what", "about", "with"]);

function hasNoTopic(question) {
  return getWords(question).every((word) => plainWords.has(word));
}

const usePattern = /\b(use|using|set|setting|change|configure|control|pass|apply)\b/i;
const listPattern = /\b(which|what|all|any|other)\s+(functions?|tools?|methods?)\b|\b(functions?|tools?)\s+(can|that|which|with|have|has|take|takes|accept|support|use)\b/i;

function findOptionOwners(question, document, { exact = false } = {}) {
  const shortForms = { minimum: "min", maximum: "max" };
  const words = getWords(question.replace(/([a-z])([A-Z])/g, "$1_$2").replace(/_/g, " ")).map((word) => shortForms[word] ?? word);
  const owners = new Map();
  for (const entry of document.entries) {
    if (entry.kind !== "function") continue;
    for (const key of listValueKeys(entry, document)) {
      const name = key.item.languages.python.name;
      if (!key.isOption || rangeKeys.has(name)) continue;
      if (!owners.has(name)) owners.set(name, []);
      if (!owners.get(name).includes(entry)) owners.get(name).push(entry);
    }
  }
  for (let size = 3; size >= 1; size -= 1) {
    for (let start = 0; start + size <= words.length; start += 1) {
      const phrase = words.slice(start, start + size).join("_");
      for (const name of exact ? [phrase] : [phrase, phrase.replace(/s$/, ""), `${phrase}s`]) {
        if (owners.has(name)) return { option: name, entries: owners.get(name) };
      }
    }
  }
  return null;
}

function makeLink(entry, language) {
  if (!entry) return { entryName: "overview", label: "Overview" };
  return { entryName: entry.name, label: getDisplayName(entry, language) };
}

function getUsageFollowUps(entry, variant) {
  if (hasVariants(entry.name)) {
    return listVariants(entry.name)
      .filter((item) => item.form !== variant?.form)
      .slice(0, 2)
      .map((item) => ({ label: item.label, question: `${item.label} with ${entry.name}` }));
  }
  return getFunctionFollowUps(entry.name).slice(0, 2);
}

export function getStarters() {
  return starters;
}

function rateUsage({ top, second, margin, named, chosenByUser, continued, hasValues }) {
  if (named || chosenByUser) return "high";
  if (continued) return hasValues ? "high" : "medium";
  const gap = top - (second ?? -Infinity);
  if (top >= limits.highScore && gap >= limits.highGap && margin >= limits.highMargin) return "high";
  if (top < limits.minScore || gap < limits.lowGap) return "low";
  return "medium";
}

function rateGuide(byKeywords, score) {
  if (byKeywords) return "high";
  return score >= limits.guideScore + limits.guideLead ? "high" : "medium";
}

export function createAsker({ embed, index, document }) {
  const readValues = embed ? createValueReader(embed) : null;
  const fixTypos = createTypoFixer(document);

  async function embedQuestion(asked) {
    const vectors = [await embed(asked)];
    const withSynonyms = applySynonyms(asked);
    if (withSynonyms !== asked) vectors.push(await embed(withSynonyms));
    return vectors;
  }

  async function finishUsage(asked, entries, scores, extra) {
    const leading = entries[0];
    const variant = findVariant(asked, leading.name);
    const read = readValues ? await readValues(asked, leading, document, getTemplateFunctions(leading, document)) : { values: { function: leading.name }, templateValues: {} };
    const hasValues = Boolean(variant) || Object.keys(read.values).length > 1 || Object.keys(read.templateValues).length > 0;
    const confidence = rateUsage({ ...scores, named: extra.named, chosenByUser: extra.chosenByUser, continued: extra.continued, hasValues });
    return {
      ...extra,
      ...scores,
      reason: { route: "usage", confidence },
      values: read.values,
      templateValues: read.templateValues,
      helperMention: read.helperMention,
      mentioned: read.mentioned,
      entries,
      variant,
      link: makeLink(leading, extra.language),
      followUps: getUsageFollowUps(leading, variant),
      alternatives: entries.slice(1).map((entry) => ({ entryName: entry.name, label: getDisplayName(entry, extra.language) })),
    };
  }

  function guideResult(item, confidence, language, extra = {}) {
    const entries = item.functions.map((name) => findByName(document, name)).filter(Boolean);
    const linked = findByName(document, item.link) ?? findGroup(document, item.link);
    const link = linked ? { entryName: linked.name, label: linked.kind ? getDisplayName(linked, language) : linked.name } : { entryName: "overview", label: "Overview" };
    const amount = extra.question?.match(numberValuePattern)?.[0];
    const fill = (text) => (amount ? text.replaceAll("{value}", amount) : text.replaceAll(" {value}", "").replaceAll("{value}", ""));
    const followUps = item.followUps.map((followUp) => ({ label: fill(followUp.label), question: followUp.question ? fill(followUp.question) : undefined }));
    return { ...extra, reason: { route: "guide", confidence }, guide: item, answer: fill(amount ? item.answer : (item.answerWithout ?? item.answer)), entries, values: null, link, followUps };
  }

  function refusedResult(extra = {}) {
    return { ...extra, reason: { route: "refused", confidence: "low" }, unknown: true, entries: [], values: null, followUps: starters };
  }

  const getExplainedEntries = (explanation) => (explanation.type === "entry" && explanation.entry.kind === "function" ? [explanation.entry] : []);
  const getExplainedOption = (explanation) => (explanation.type === "field" && listOptionFunctions(explanation.term, document).length > 0 ? explanation.term : null);

  const asker = async (question, language, previous, page) => {
    const raw = question.slice(0, isCommand(question) ? limits.code : limits.question);
    if (isCommand(raw)) {
      const command = runCommand(raw, document, language);
      return { reason: { route: "command", confidence: "high" }, command, entries: command.entry ? [command.entry] : (command.review?.entries ?? []), values: null, followUps: [] };
    }
    const original = raw.replace(/\bquant\s*[·.\-]?\s*ion\b/gi, "quantion");
    const asked = looksLikeCode(original) ? original : fixTypos(original);
    if (looksLikeCode(asked)) {
      const review = reviewCode(original, document, language);
      if (review.entries.length > 0) return { reason: { route: "review", confidence: "high" }, review, entries: review.entries, values: null, link: makeLink(review.entries[0], language), followUps: [] };
    }
    const listResult = (owners) => {
      const followUps = owners.entries.map((entry) => ({ label: getDisplayName(entry, language), question: `How do I use ${entry.name} with ${owners.option}?` }));
      return { reason: { route: "list", confidence: "high" }, list: owners, option: owners.option, entries: owners.entries, values: null, followUps };
    };
    const owners = listPattern.test(asked) ? findOptionOwners(asked, document) : null;
    if (owners) return listResult(owners);
    const namedFunctionEarly = document.entries.find((entry) => entry.kind === "function" && namesFunction(entry, asked));
    if (!namedFunctionEarly && !/\d/.test(asked) && !placementPattern.test(asked)) {
      const named = usePattern.test(asked) ? findOptionOwners(asked, document, { exact: true }) : null;
      if (named && named.entries.length > 1) return listResult(named);
      if (!named && previous?.option && refersBack(asked) && !isMeaningQuestion(asked)) return listResult({ option: previous.option, entries: listOptionFunctions(previous.option, document) });
    }
    const keywordItems = matchGuideKeywords(asked);
    const namedEntry = findNamedEntry(asked, document);
    const isFragment = continuesTopic(asked) && (continuationPattern.test(asked) || (keywordItems.length === 0 && !namedEntry));
    const isContinuation = Boolean(previous) && (isFragment || refersBack(asked));
    if (!embed) {
      const explanation = explainTerm(asked, document, getContextEntries(document, [page, previous?.functionName], findOptionsType));
      if (explanation) return { reason: { route: "definition", confidence: "medium" }, explanation, option: getExplainedOption(explanation), entries: getExplainedEntries(explanation), values: null, followUps: [] };
      if (keywordItems.length > 0 && namedEntry?.kind !== "function") return guideResult(keywordItems[0], "high", language);
      return { ...refusedResult(), noModel: true };
    }
    const vectors = await embedQuestion(asked);
    const questionVector = vectors[0];
    const contextEntries = getContextEntries(document, [page, previous?.functionName], findOptionsType);
    const guideScores = getGuideScores(questionVector, index);
    const allowsNamed = (item) => namedEntry?.kind !== "function" || item.functions.includes(namedEntry.name);
    const firstGuide = pickGuideItem(guideScores, keywordItems.filter((item) => item.first && allowsNamed(item)));
    if (firstGuide && !isContinuation) return guideResult(firstGuide.item, "high", language, { questionVector });
    if (placementPattern.test(asked)) {
      const contextFunction = previous?.functionName ? findByName(document, previous.functionName) : null;
      const searched = pickEntries(addNameBonus(rankWithVectors(vectors, index), asked, document), document)[0];
      const placement = findPlacement(asked, contextFunction?.kind === "function" ? contextFunction : searched, document);
      if (placement) return { questionVector, reason: { route: "placement", confidence: "high" }, placement, entries: [placement.entry], values: null, link: makeLink(placement.owner, language), followUps: getUsageFollowUps(placement.entry, null) };
    }
    const rawRanked = rankWithVectors(vectors, index);
    const rawTopScore = rawRanked[0].score;
    const descriptiveTopScore = rankWithVectors(vectors, index, { descriptive: true })[0].score;
    const offTopicScore = Math.max(...vectors.map((vector) => getOffTopicScore(vector, index)));
    const explanation = explainTerm(asked, document, contextEntries);
    const isExplanationOffTopic = explanation && !isContinuation && !isAboutPage(explanation, page) && offTopicScore > rawTopScore - limits.topicMargin;
    if (isExplanationOffTopic) return refusedResult({ questionVector, rawTopScore, offTopicScore });
    if (explanation) {
      const confidence = explanation.type === "entry" || isAboutPage(explanation, page) ? "high" : "medium";
      return { questionVector, reason: { route: "definition", confidence }, explanation, option: getExplainedOption(explanation), entries: getExplainedEntries(explanation), values: null, followUps: [] };
    }
    const allowedItems = keywordItems.filter((item) => allowsNamed(item) && !(item.numbers && namesFunction(namedEntry, asked)));
    if (allowedItems.length > 0 && !isContinuation) {
      const best = pickGuideItem(guideScores, allowedItems);
      return guideResult(best.item, "high", language, { questionVector, guideScore: best.score, question: asked });
    }
    const searchVectors = previous && isContinuation ? vectors.map((vector) => blendVectors(vector, previous.questionVector, limits.earlierWeight)) : vectors;
    const scored = addKeyBonus(addNameBonus(rankWithVectors(searchVectors, index), asked, document), asked, document);
    const ranked = pickEntries(scored, document);
    const topScore = scored[0].score;
    const margin = descriptiveTopScore - offTopicScore;
    const isOffTopic = !hasDomainWord(original) && (margin < limits.topicMargin || descriptiveTopScore < limits.minScore);
    const isPlainWord = namedEntry?.kind === "function" && !isCodeForm(namedEntry, asked);
    const namedFunction = namedEntry?.kind === "function" && (namedEntry.name.includes("_") ? !isPlainWord : !(isPlainWord && isOffTopic)) ? namedEntry : null;
    const guideBest = !namedFunction && !isContinuation ? pickGuideItem(guideScores, guideItems.filter((item) => !item.numbers || (keywordItems.includes(item) && !namesFunction(namedEntry, asked)))) : null;
    if (!namedFunction && isOffTopic && !isContinuation) {
      if (guideBest && guideBest.score >= limits.guideScore + limits.guideLead) return guideResult(guideBest.item, rateGuide(false, guideBest.score), language, { questionVector, guideScore: guideBest.score });
      return refusedResult({ questionVector, topScore, rawTopScore, descriptiveTopScore, offTopicScore });
    }
    if (guideBest && guideBest.score >= limits.guideScore && guideBest.score >= rawTopScore + limits.guideLead) {
      return guideResult(guideBest.item, rateGuide(false, guideBest.score), language, { questionVector, guideScore: guideBest.score });
    }
    const previousFunction = previous?.functionName ? findByName(document, previous.functionName) : null;
    const earlierFunction = previous && isContinuation && previousFunction?.kind === "function" && (isFragment || hasNoTopic(asked) || ranked.includes(previousFunction)) ? previousFunction : null;
    const leading = namedFunction ?? earlierFunction;
    const entries = leading ? [leading, ...ranked.filter((entry) => entry !== leading)].slice(0, limits.entries) : ranked;
    const functionScores = getFunctionScores(scored, document);
    const second = [...functionScores.entries()].find(([name]) => name !== entries[0].name)?.[1] ?? null;
    const scores = { top: functionScores.get(entries[0].name) ?? topScore, second, margin, topScore, rawTopScore, descriptiveTopScore, offTopicScore };
    return finishUsage(asked, entries, scores, { questionVector, language, named: Boolean(namedFunction), continued: Boolean(earlierFunction), ranking: scored.slice(0, 5), example: findExampleEntry(entries, language) });
  };

  asker.reread = async (question, result, entryName, language) => {
    const entry = findByName(document, entryName);
    if (!entry || entry.kind !== "function") return result;
    const asked = fixTypos(question.slice(0, limits.question));
    const entries = [entry, ...result.entries.filter((item) => item !== entry)].slice(0, limits.entries);
    const scores = { top: result.top, second: result.second, margin: result.margin, topScore: result.topScore, rawTopScore: result.rawTopScore, descriptiveTopScore: result.descriptiveTopScore, offTopicScore: result.offTopicScore };
    return finishUsage(asked, entries, scores, { questionVector: result.questionVector, language, named: result.named, chosenByUser: true, example: findExampleEntry(entries, language) });
  };

  return asker;
}
