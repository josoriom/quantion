import { limits } from "./models.js";
import { listValueKeys } from "./snippet.js";

const numberPattern = /(?<![\w.])-?\d+(?:\.\d+)?(?!\w|\.\d|-[A-Za-z])/g;
const percentPattern = /^\s*(%|percent|per cent)/i;
const fractionMeaning = /\(0[–-]1\)|fraction|share of/i;
const timeKeys = new Set(["rt", "from", "to"]);
const largestMinutes = 60;
const smallestMz = 20;
const largestPpm = 100;
const largestTolerance = 5;
const offPattern = /\b(turn off|switch off|disable|disabled|without|no|not|don't|skip|stop)\b/i;
const onPattern = /\b(turn on|switch on|enable|enabled|activate)\b/i;
const stopWords = new Set(["to", "of", "the", "a", "an", "and", "in"]);
const limitWords = new Set(["above", "below", "over", "under", "least", "most", "minimum", "maximum", "min", "max", "more", "less", "fewer", "higher", "lower", "bigger", "smaller", "larger", "limit", "threshold", "cutoff", "small", "tiny", "big", "large", "drop", "ignore", "remove", "only"]);
const fillerWords = new Set(["with", "that", "this", "these", "those", "from", "into", "when", "have", "does", "what", "which", "where", "there", "their", "your", "about", "using", "want", "need", "should", "could", "would", "make", "each", "only", "than", "then", "some", "will"]);
const ignoredTokens = new Set(["min", "max", "auto", "use", "of", "the", "peak", "peaks", "to", "from"]);
const rangeUnit = String.raw`(?:m\/z|mz|minutes?|mins?|seconds?|secs?|rt)`;
const rangePattern = new RegExp(String.raw`\b(?:between|from)\s+(?:${rangeUnit}\s+)?(-?\d+(?:\.\d+)?)\s*(?:${rangeUnit}\s*)?(?:and|to|until|till|up to|-)\s*(?:${rangeUnit}\s+)?(-?\d+(?:\.\d+)?)`, "gi");
const rangePairs = [
  ["from", "to"],
  ["start", "end"],
];
const numberUnitPattern = /\d\s*(minutes?|mins?|seconds?|secs?)\b/gi;
const plainWordKeys = new Set(["from", "to", "start", "end", "level", "range", "out", "shape", "points", "noise", "file", "files", "matches", "model", "library"]);
const countWords = new Set(["targets", "target", "files", "file", "samples", "sample", "features", "feature", "peaks", "peak", "scans", "injections", "runs", "batches", "transitions", "spectra", "folders"]);
const nearWords = new Set(["near", "around", "at", "closest"]);
const unitHints = [
  { words: ["min", "mins", "minute", "minutes", "sec", "secs", "second", "seconds", "s"], tokens: ["rt", "from", "to", "range"], meaning: /time|minute/i },
  { words: ["point", "points"], tokens: ["points"], meaning: /points/i },
  { words: ["ppm"], tokens: ["ppm"], meaning: /ppm/i },
  { words: ["da"], tokens: ["mz"], meaning: /\bDa\b/ },
  { words: ["percent"], tokens: ["relative", "frequency"], meaning: /percent|relative|fraction/i },
];

const synonyms = [
  { words: ["monoisotopic mass", "exact mass", "accurate mass", "masses", "mass", "m/z"], to: "mz" },
  { words: ["retention times", "retention time", "elution time"], to: "rt" },
  { words: ["areas", "area"], to: "integral" },
  { words: ["heights", "height", "abundance"], to: "intensity" },
  { words: ["spectra", "spectrum"], to: "scans" },
  { words: ["chromatograms", "chromatogram", "xic"], to: "eic" },
  { words: ["compounds", "compound", "analytes", "analyte", "metabolites", "metabolite", "peptides", "peptide", "lipids", "lipid"], to: "targets" },
  { words: ["background", "drift"], to: "baseline" },
  { words: ["cores", "cpus"], to: "threads" },
  { words: ["read", "open", "load", "import"], to: "parse" },
  { words: ["save", "export", "convert"], to: "write" },
  { words: ["fetch"], to: "get" },
  { words: ["picking", "pick", "detect", "locate"], to: "find" },
  { words: ["close", "free", "release", "clean up"], to: "dispose" },
  { words: ["cvparams", "userparams", "attributes", "attribute"], to: "metadata" },
  { words: ["simulate", "synthetic", "generate"], to: "draw" },
  { words: ["srm", "mrm"], to: "transitions" },
];

const patterns = synonyms.flatMap(({ words, to }) =>
  words.map((word) => ({ pattern: new RegExp(`(?<![\\w/])${word.replace("/", "\\/")}(?![\\w/])`, "gi"), to })),
);

function applySynonyms(text) {
  let clean = text;
  for (const { pattern, to } of patterns) clean = clean.replace(pattern, to);
  return clean;
}

function getDotProduct(left, right) {
  let score = 0;
  for (let position = 0; position < left.length; position += 1) score += left[position] * right[position];
  return score;
}

export function getWords(text) {
  return applySynonyms(text).toLowerCase().match(/[a-z0-9]+/g) ?? [];
}

function sharesStem(left, right) {
  const length = Math.min(5, left.length, right.length);
  if (length < 2) return false;
  if (left.length < 4 || right.length < 4) return left === right;
  return left.slice(0, length) === right.slice(0, length);
}

function isAcronym(token, words) {
  if (token.length < 2 || token.length > 4) return false;
  const initials = words.filter((word) => !stopWords.has(word)).map((word) => word[0]).join("");
  return initials.includes(token);
}

function hasExactName(key, words) {
  if (!key) return false;
  return key.item.languages.python.name.split("_").every((token) => ignoredTokens.has(token) || words.includes(token));
}

export function hasNameWord(name, words) {
  for (const token of name.split("_")) {
    if (ignoredTokens.has(token)) continue;
    if (words.some((word) => sharesStem(word, token)) || isAcronym(token, words)) return true;
  }
  return false;
}

function getUnitBonus(key, afterWords, beforeWords) {
  const tokens = key.item.languages.python.name.split("_");
  if (beforeWords.some((word) => nearWords.has(word)) && tokens.includes("rt")) return 0.4;
  const lastWord = beforeWords[beforeWords.length - 1];
  if (lastWord && key.item.languages.python.name === lastWord) return 0.5;
  if (lastWord && tokens.some((token) => !ignoredTokens.has(token) && sharesStem(lastWord, token))) return 0.4;
  for (const hint of unitHints) {
    if (!afterWords.some((word) => hint.words.includes(word))) continue;
    if (tokens.some((token) => hint.tokens.includes(token)) || hint.meaning.test(key.item.meaning)) return 0.4;
  }
  return 0;
}

function getMeaningBonus(key, words) {
  let bonus = 0;
  const meaningWords = new Set(getWords(key.item.meaning).filter((word) => word.length >= 4 && !stopWords.has(word) && !fillerWords.has(word) && !ignoredTokens.has(word)));
  for (const word of meaningWords) {
    if (words.some((other) => other.length >= 4 && sharesStem(other, word))) bonus += 0.1;
  }
  return Math.min(bonus, 0.3);
}

function describeKey(key) {
  return `${key.item.languages.python.name.replace(/_/g, " ")}: ${key.item.meaning}`;
}

function getContext(question, start, end) {
  const before = question.slice(0, start).trim().split(/\s+/).slice(-5).join(" ");
  const after = question.slice(end).trim().split(/\s+/).slice(0, 3).join(" ");
  return {
    text: `${before} ${after}`.trim(),
    afterWords: getWords(question.slice(end)).slice(0, 2),
    beforeWords: getWords(question.slice(0, start)).slice(-2),
  };
}

function fitsNumber(kind, value) {
  if (kind.kind === "integer") return Number.isInteger(value) && value >= 0;
  return kind.kind === "number";
}

function isImpossible(key, value) {
  const name = key.item.languages.python.name;
  if (timeKeys.has(name) && value > largestMinutes * 60) return true;
  if (name === "ppm_tolerance" && value > largestPpm * 10) return true;
  if (/tolerance$|^step_size$/.test(name) && name !== "ppm_tolerance" && value > largestTolerance * 10) return true;
  return false;
}

function getRangePenalty(key, value) {
  const name = key.item.languages.python.name;
  if (timeKeys.has(name) && value > largestMinutes) return -0.3;
  if (name === "mz" && value < smallestMz) return -0.3;
  if (name === "mz" && value >= 50 && !Number.isInteger(value)) return 0.1;
  if (name === "ppm_tolerance" && value > largestPpm) return -0.3;
  if (/tolerance$|^step_size$/.test(name) && value > largestTolerance) return -0.3;
  return 0;
}

function toFraction(key, value, after) {
  if (!percentPattern.test(after)) return value;
  if (fractionMeaning.test(key.item.meaning)) return value / 100;
  return null;
}

export function createValueReader(embed) {
  const cache = new Map();

  async function embedOnce(text) {
    if (!cache.has(text)) cache.set(text, await embed(text));
    return cache.get(text);
  }

  async function scoreKeys(text, keys, afterWords = [], beforeWords = [], subjectWords = []) {
    const vector = await embedOnce(text);
    const isSubject = (word) => subjectWords.some((subject) => sharesStem(word, subject) || (word.length >= 4 && subject.includes(word.replace(/s$/, ""))));
    const allWords = getWords(text);
    const hasLimitWord = allWords.some((word) => limitWords.has(word));
    const words = getWords(text).filter((word) => !isSubject(word));
    const scored = [];
    for (const key of keys) {
      const similarity = getDotProduct(vector, await embedOnce(describeKey(key)));
      const nameBonus = hasNameWord(key.item.languages.python.name, words) ? 0.3 : 0;
      const meaningBonus = getMeaningBonus(key, words);
      const isLimit = /^(min|max)_/.test(key.item.languages.python.name);
      const hasEvidence = (nameBonus > 0 || meaningBonus > 0) && (!isLimit || hasLimitWord);
      scored.push({ key, score: similarity + nameBonus + meaningBonus + getUnitBonus(key, afterWords, beforeWords), hasEvidence });
    }
    return scored;
  }

  function keyId(key) {
    return `${key.owner.name}.${key.item.languages.python.name}`;
  }

  function store(result, key, value) {
    const name = key.item.languages.python.name;
    if (key.isMain) result.values[name] = value;
    else result.templateValues[key.owner.name] = { ...(result.templateValues[key.owner.name] ?? {}), [name]: value };
  }

  async function readRanges(question, keys, result, used, placed) {
    for (const match of question.matchAll(rangePattern)) {
      const context = getContext(question, match.index, match.index + match[0].length);
      let best = null;
      for (const owner of [...new Set(keys.map((key) => key.owner))]) {
        for (const [startName, endName] of rangePairs) {
          const startKey = keys.find((key) => key.owner === owner && key.item.languages.python.name === startName && key.kind.kind === "number");
          const endKey = keys.find((key) => key.owner === owner && key.item.languages.python.name === endName && key.kind.kind === "number");
          if (!startKey || !endKey || used.has(keyId(startKey))) continue;
          const [scored] = await scoreKeys(`${match[0]} ${context.text}`, [startKey], context.afterWords);
          const score = scored.score + (startKey.isMain ? limits.mainKeyBonus : 0);
          if (!best || score > best.score) best = { startKey, endKey, score };
        }
      }
      placed.add(match.index + match[0].indexOf(match[1]));
      placed.add(match.index + match[0].lastIndexOf(match[2]));
      if (!best) continue;
      store(result, best.startKey, Number(match[1]));
      store(result, best.endKey, Number(match[2]));
      used.add(keyId(best.startKey));
      used.add(keyId(best.endKey));
    }
  }

  return async (question, entry, document, helpers = []) => {
    const keys = listValueKeys(entry, document).map((key) => ({ ...key, owner: entry, isMain: true }));
    for (const helper of helpers) {
      for (const key of listValueKeys(helper, document)) {
        if (key.kind.kind === "number" || key.kind.kind === "integer") keys.push({ ...key, owner: helper, isMain: false });
      }
    }
    const result = { values: { function: entry.name }, templateValues: {} };
    const values = result.values;
    const used = new Set();

    const placedNumbers = new Set();
    await readRanges(question, keys, result, used, placedNumbers);
    const candidates = [];
    for (const match of question.matchAll(numberPattern)) {
      if (placedNumbers.has(match.index)) continue;
      const value = Number(match[0]);
      const numberKeys = keys.filter((key) => fitsNumber(key.kind, value) && !isImpossible(key, value) && !used.has(keyId(key)));
      const context = getContext(question, match.index, match.index + match[0].length);
      const after = question.slice(match.index + match[0].length);
      const counted = context.afterWords[0];
      if (counted && countWords.has(counted) && !numberKeys.some((key) => key.item.languages.python.name.split("_").includes(counted))) continue;
      for (const item of await scoreKeys(context.text, numberKeys, context.afterWords, context.beforeWords)) {
        candidates.push({ ...item, score: item.score + (item.key.isMain ? limits.mainKeyBonus : 0) + getRangePenalty(item.key, value), value, match: match.index, after });
      }
    }
    candidates.sort((left, right) => right.score - left.score);
    for (const candidate of candidates) {
      if (used.has(keyId(candidate.key)) || placedNumbers.has(candidate.match)) continue;
      const value = toFraction(candidate.key, candidate.value, candidate.after);
      placedNumbers.add(candidate.match);
      if (value === null) continue;
      store(result, candidate.key, value);
      used.add(keyId(candidate.key));
    }

    const words = getWords(question.replace(numberUnitPattern, " "));
    for (const key of keys) {
      if (!key.isMain || key.kind.kind !== "enum") continue;
      for (const member of key.kind.entry.fields ?? []) {
        const memberName = member.languages.python.name;
        if (!words.includes(memberName.toLowerCase())) continue;
        values[key.item.languages.python.name] = memberName;
        used.add(key.item.languages.python.name);
      }
    }

    const optionKeys = keys.filter((key) => key.isMain && key.isOption && !used.has(keyId(key)));
    const helperMention = keys.find((key) => !key.isMain && !used.has(keyId(key)) && !plainWordKeys.has(key.item.languages.python.name) && words.includes(key.item.languages.python.name));
    const named = helperMention ? { owner: helperMention.owner, item: helperMention.item } : null;
    const subjectWords = entry.name.split("_");
    const ranked = (await scoreKeys(question, optionKeys, [], [], subjectWords)).sort((left, right) => right.score - left.score);
    const best = ranked[0];
    const nothingPlaced = Object.keys(values).length === 1 && Object.keys(result.templateValues).length === 0;
    const isSwitch = offPattern.test(question) || onPattern.test(question) || (nothingPlaced && hasExactName(best?.key, words));
    if (best?.key.kind.kind === "boolean" && best.score >= 0.5 && isSwitch) {
      values[best.key.item.languages.python.name] = !offPattern.test(question);
      return { ...result, mentioned: null, helperMention: named };
    }
    const mentioned = best && best.hasEvidence && best.score >= 0.6 && Object.keys(values).length === 1 && Object.keys(result.templateValues).length === 0 ? best.key.item : null;
    return { ...result, mentioned, helperMention: named };
  };
}
