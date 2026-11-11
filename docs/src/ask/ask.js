import { getDisplayName } from "../names.js";
import { isCommand, runCommand } from "./commands.js";
import { findVariant, getFunctionFollowUps, guideItems, hasVariants, listVariants, starters } from "./guide.js";
import { limits, models } from "./models.js";
import { listOptionFunctions, listValueKeys } from "./snippet.js";
import { looksLikeCode, reviewCode } from "./review.js";
import { createValueReader } from "./values.js";

function findByName(document, name) {
  for (const entry of document.entries) {
    if (entry.name === name) return entry;
  }
  return null;
}

function findGroup(document, name) {
  return (document.groups ?? []).find((group) => group.name === name) ?? null;
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

function splitSentences(text) {
  return text.split(/(?<=\.)\s+/).filter((sentence) => sentence.length > 3);
}

export function trimAnswer(text) {
  const clean = text.replace(/\s+/g, " ").trim();
  if (clean.length <= limits.answer) return clean;
  const head = clean.slice(0, limits.answer);
  const sentenceEnd = head.lastIndexOf(". ");
  if (sentenceEnd > limits.answer / 3) return head.slice(0, sentenceEnd + 1);
  return `${head.slice(0, head.lastIndexOf(" "))}…`;
}

export function createClassifier({ labels, scale, dimension }, buffer) {
  const numbers = new Float32Array(buffer);
  const bias = numbers.subarray(labels.length * dimension);
  return (vector) => {
    const scores = labels.map((label, row) => {
      let score = 0;
      for (let column = 0; column < dimension; column += 1) score += numbers[row * dimension + column] * vector[column];
      return score * scale + bias[row];
    });
    const largest = Math.max(...scores);
    const exponents = scores.map((score) => Math.exp(score - largest));
    const total = exponents.reduce((sum, value) => sum + value, 0);
    return labels.map((id, row) => ({ id, probability: exponents[row] / total })).sort((left, right) => right.probability - left.probability);
  };
}

function readAnswerId(id) {
  const split = id.indexOf(":");
  if (split < 0) return { route: id, target: "" };
  return { route: id.slice(0, split), target: id.slice(split + 1) };
}

export function findExampleEntry(entries, language) {
  for (const entry of entries) {
    if (entry.languages[language]?.example) return entry;
  }
  return null;
}

const localRuntimes = new WeakSet();

function useLocalRuntime(wasm, runtimeFiles) {
  if (!wasm || !runtimeFiles || localRuntimes.has(wasm)) return;
  const current = typeof wasm.wasmPaths === "object" && wasm.wasmPaths ? Object.values(wasm.wasmPaths) : [];
  if (current.some((address) => String(address).startsWith("blob:") || Object.values(runtimeFiles).includes(address))) {
    localRuntimes.add(wasm);
    return;
  }
  const paths = typeof wasm.wasmPaths === "object" && wasm.wasmPaths ? wasm.wasmPaths : { mjs: "ort-wasm-simd-threaded.asyncify.mjs", wasm: "ort-wasm-simd-threaded.asyncify.wasm" };
  const local = {};
  for (const [kind, address] of Object.entries(paths)) {
    const name = String(address).split("/").pop();
    if (!runtimeFiles[name]) throw new Error(`No local ONNX runtime file ${name}`);
    local[kind] = runtimeFiles[name];
  }
  wasm.wasmPaths = local;
  localRuntimes.add(wasm);
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
    const output = await extractor(text, { pooling: models.embedder.pooling, normalize: true });
    return Array.from(output.data);
  };
}

function blendVectors(current, earlier, weight) {
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
  else if (helpers.length === 0 && result.mentioned) {
    text = `Use ${functionName} and set its option \`${result.mentioned.languages[language].name}\`: ${result.mentioned.meaning}`;
    const address = result.mentioned.languages[language].default?.match(/^"(https?:\/\/[^"]+)"$/)?.[1];
    if (address) text += ` By default it is ${address}.`;
  }
  else text = `Use ${functionName}: ${entry.summary}`;
  if (helpers.length > 0) text += ` In the template, use ${helpers.join(" and ")}.`;
  else if (result.helperMention) {
    const { owner, item } = result.helperMention;
    text += ` The \`${item.languages[language].name}\` goes in \`${getDisplayName(owner, language)}\`: ${item.meaning}`;
  }
  return trimAnswer(text);
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

function isCodeForm(entry, text) {
  if (!entry.name.includes("_")) return new RegExp(`\\b${entry.name}\\(|\\.${entry.name}\\b`, "i").test(text);
  const snake = entry.name.replace(/s$/, "");
  const camel = getDisplayName(entry, "js").replace(/s$/, "");
  return new RegExp(`(^|[^\\w.])(${snake}|${camel})s?(\\(|\\b)`, "i").test(text);
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

const protonMass = 1.007276;

export function readIonMz(question) {
  const text = question.toLowerCase();
  const negative = /\bneg(ative)?\b|\[m\s*-\s*h\]|esi\s*-/.test(text);
  const positive = /\bpos(itive)?\b|\[m\s*\+\s*h\]|esi\s*\+/.test(text);
  if (negative === positive) return null;
  if (!/\b(mass|monoisotopic|neutral|molecular weight|mw)\b/.test(text)) return null;
  const numbers = (text.match(/\d+\.\d+|\d{2,}/g) ?? []).map(Number).filter((value) => value >= 20 && value <= 5000);
  if (numbers.length !== 1) return null;
  const mass = numbers[0];
  const decimals = Math.max(4, (String(mass).split(".")[1] ?? "").length);
  const mz = (negative ? mass - protonMass : mass + protonMass).toFixed(decimals);
  return negative
    ? { mode: "negative", name: "deprotonated molecule", formula: "[M−H]−", sign: "−", mass, mz }
    : { mode: "positive", name: "protonated molecule", formula: "[M+H]+", sign: "+", mass, mz };
}

function getPlainName(item) {
  return (item.languages.python?.name ?? "").replace(/"/g, "");
}

function findItem(entry, name, document) {
  const items = [...(entry.arguments ?? []), ...(entry.fields ?? []), ...(findOptionsType(entry, document)?.fields ?? [])];
  return items.find((item) => getPlainName(item) === name) ?? items.find((item) => getPlainName(item).toLowerCase() === name.toLowerCase()) ?? null;
}

function getOwnerName(target) {
  return target.includes("@") ? target.split("@")[1] : target;
}

function rateProbability(probability) {
  if (probability >= limits.highProbability) return "high";
  if (probability >= limits.lowProbability) return "medium";
  return "low";
}

export function createAsker({ embed, classify, document }) {
  const readValues = embed ? createValueReader(embed) : null;

  async function finishUsage(asked, entries, confidence, extra) {
    const leading = entries[0];
    const variant = findVariant(asked, leading.name);
    const read = readValues ? await readValues(asked, leading, document, getTemplateFunctions(leading, document)) : { values: { function: leading.name }, templateValues: {} };
    return {
      ...extra,
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

  function listResult(option, entries, language) {
    const followUps = entries.map((entry) => ({ label: getDisplayName(entry, language), question: `How do I use ${entry.name} with ${option}?` }));
    return { reason: { route: "list", confidence: "high" }, list: { option, entries }, option, entries, values: null, followUps };
  }

  function findFunction(target) {
    const owner = findByName(document, getOwnerName(target));
    if (!owner) return null;
    if (owner.kind === "function") return owner;
    return document.entries.find((entry) => entry.kind === "function" && findOptionsType(entry, document) === owner) ?? null;
  }

  function listFunctions(ranked) {
    const found = [];
    for (const { id } of ranked) {
      const entry = findFunction(readAnswerId(id).target);
      if (entry && !found.includes(entry)) found.push(entry);
      if (found.length === limits.entries) break;
    }
    return found;
  }

  async function answerFromId(id, probability, ranked, asked, questionVector, language) {
    const { route, target } = readAnswerId(id);
    const confidence = rateProbability(probability);
    const extra = { questionVector, probability, answerId: id };
    if (route === "refuse") return refusedResult(extra);
    if (route === "unsupported") {
      return { ...extra, reason: { route: "unsupported", confidence }, faq: { note: "Quant·ion does not do that.", route, target: "" }, entries: [], values: null, link: null, followUps: starters };
    }
    if (route === "guide") {
      const item = guideItems.find((candidate) => candidate.id === target);
      return item ? guideResult(item, confidence, language, { ...extra, question: asked }) : null;
    }
    if (route === "list") {
      const entries = listOptionFunctions(target, document);
      return entries.length > 0 ? { ...extra, ...listResult(target, entries, language) } : null;
    }
    if (route === "definition" && !target.includes("@") && findGroup(document, target)) {
      return { ...extra, reason: { route: "definition", confidence }, explanation: { type: "group", group: findGroup(document, target) }, entries: [], values: null, followUps: [] };
    }
    const owner = findByName(document, getOwnerName(target));
    if (!owner) return null;
    if (route === "usage" && owner.kind === "function") {
      const entries = [owner, ...listFunctions(ranked).filter((entry) => entry !== owner)].slice(0, limits.entries);
      return finishUsage(asked, entries, confidence, { ...extra, language, example: findExampleEntry(entries, language) });
    }
    if (route === "placement" && target.includes("@") && owner.kind === "function") {
      const field = target.split("@")[0];
      const candidates = [owner, ...getTemplateFunctions(owner, document), ...document.entries];
      for (const candidate of candidates) {
        const item = findItem(candidate, field, document);
        if (item) return { ...extra, reason: { route: "placement", confidence }, placement: { entry: owner, owner: candidate, item }, entries: [owner], values: null, link: makeLink(candidate, language), followUps: getUsageFollowUps(owner, null) };
      }
      return null;
    }
    if (route === "definition") {
      const field = target.includes("@") ? target.split("@")[0] : null;
      const item = field ? findItem(owner, field, document) : null;
      if (field && !item) return null;
      const explanation = item ? { type: "field", term: field, groups: [{ meaning: item.meaning, matches: [{ entry: owner, item }], isHere: true }] } : { type: "entry", entry: owner };
      const option = explanation.type === "field" && listOptionFunctions(explanation.term, document).length > 0 ? explanation.term : null;
      const entries = explanation.type === "entry" && owner.kind === "function" ? [owner] : [];
      return { ...extra, reason: { route: "definition", confidence }, explanation, option, entries, values: null, followUps: [] };
    }
    return null;
  }

  function findTypedFunction(asked) {
    return document.entries.find((entry) => entry.kind === "function" && isCodeForm(entry, asked)) ?? null;
  }

  async function answerIonMz(asked, language) {
    const ion = readIonMz(asked);
    const entry = findByName(document, "calculate_eic");
    if (!ion || !entry) return null;
    const result = await finishUsage(asked, [entry], "high", { language, example: findExampleEntry([entry], language) });
    const note = `In ${ion.mode} mode, the ${ion.name} ${ion.formula} has m/z ${ion.mass} ${ion.sign} 1.00728 = ${ion.mz}. Use ${ion.mz} as mz in calculate_eic or in your targets.`;
    return { ...result, values: { function: entry.name, mz: Number(ion.mz) }, templateValues: {}, faq: { note, route: "usage", target: entry.name, score: 1 } };
  }

  const asker = async (question, language, previous) => {
    const raw = question.slice(0, isCommand(question) ? limits.code : limits.question);
    if (isCommand(raw)) {
      const command = runCommand(raw, document, language);
      return { reason: { route: "command", confidence: "high" }, command, entries: command.entry ? [command.entry] : (command.review?.entries ?? []), values: null, followUps: [] };
    }
    if (looksLikeCode(raw)) {
      const review = reviewCode(raw, document, language);
      if (review.entries.length > 0) return { reason: { route: "review", confidence: "high" }, review, entries: review.entries, values: null, link: makeLink(review.entries[0], language), followUps: [] };
    }
    const ionAnswer = await answerIonMz(raw, language);
    if (ionAnswer) return ionAnswer;
    if (!embed || !classify) return { ...refusedResult(), noModel: true };
    const questionVector = await embed(raw);
    let ranked = classify(questionVector);
    if (previous?.questionVector && ranked[0].probability < limits.highProbability) {
      const withEarlier = classify(blendVectors(questionVector, previous.questionVector, limits.earlierWeight));
      if (withEarlier[0].probability > ranked[0].probability && withEarlier[0].id !== "refuse") ranked = withEarlier;
    }
    const typed = findTypedFunction(raw);
    if (typed && ranked[0].id === "refuse") {
      const about = ranked.find(({ id }) => id !== "refuse" && id !== "unsupported" && findFunction(readAnswerId(id).target) === typed);
      ranked = about ? [about, ...ranked.filter((item) => item !== about)] : [{ id: `usage:${typed.name}`, probability: 1 }, ...ranked];
    }
    for (const { id, probability } of ranked.slice(0, limits.entries)) {
      const result = await answerFromId(id, probability, ranked, raw, questionVector, language);
      if (result) return result;
    }
    return refusedResult({ questionVector });
  };

  asker.reread = async (question, result, entryName, language) => {
    const entry = findByName(document, entryName);
    if (!entry || entry.kind !== "function") return result;
    const asked = question.slice(0, limits.question);
    const entries = [entry, ...result.entries.filter((item) => item !== entry)].slice(0, limits.entries);
    return finishUsage(asked, entries, "high", { questionVector: result.questionVector, language, example: findExampleEntry(entries, language) });
  };

  return asker;
}
