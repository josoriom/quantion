import { levenshteinDistance } from "str-utilities";

export const synonyms = [
  { words: ["monoisotopic mass", "exact mass", "accurate mass", "masses", "mass", "m/z"], to: "mz" },
  { words: ["retention times", "retention time", "elution time"], to: "rt" },
  { words: ["areas", "area"], to: "integral" },
  { words: ["heights", "height", "abundance"], to: "intensity" },
  { words: ["spectra", "spectrum"], to: "scans" },
  { words: ["chromatograms", "chromatogram", "xic"], to: "eic" },
  { words: ["compounds", "compound", "analytes", "analyte"], to: "targets" },
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

const domainWords = new Set(["mz", "ppm", "eic", "xic", "mzml", "ms1", "ms2", "msms", "chromatogram", "chromatograms", "srm", "mrm", "airpls", "fwhm", "emg", "isotope", "isotopes", "deisotope", "monoisotopic", "spectra", "spectrum", "lcms"]);

export function hasDomainWord(text) {
  const words = text.toLowerCase().replace(/m\/z/g, "mz").replace(/lc-ms/g, "lcms").match(/[a-z0-9]+/g) ?? [];
  return words.some((word) => domainWords.has(word));
}

export function applySynonyms(text) {
  let clean = text;
  for (const { pattern, to } of patterns) clean = clean.replace(pattern, to);
  return clean;
}

function getAllowedEdits(word) {
  if (word.length >= 8) return 2;
  if (word.length >= 5) return 1;
  return 0;
}

export function createTypoFixer(document) {
  const vocabulary = new Set();
  for (const entry of document.entries) {
    vocabulary.add(entry.name.toLowerCase());
    for (const token of entry.name.replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase().split("_")) vocabulary.add(token);
    for (const item of [...(entry.arguments ?? []), ...(entry.fields ?? [])]) {
      for (const language of Object.values(item.languages ?? {})) {
        if (!language?.name) continue;
        vocabulary.add(language.name.toLowerCase());
        for (const token of language.name.toLowerCase().split(/[_.]/)) vocabulary.add(token);
      }
    }
    if (entry.kind === "function") vocabulary.add(entry.name.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase()).toLowerCase());
  }
  for (const group of document.groups ?? []) vocabulary.add(group.name.toLowerCase());
  vocabulary.add("quantion");
  for (const { words, to } of synonyms) {
    vocabulary.add(to);
    for (const word of words) if (!word.includes(" ")) vocabulary.add(word);
  }
  const candidates = [...vocabulary].filter((word) => word.length >= 4);

  function fixWord(word) {
    const lower = word.toLowerCase();
    const allowed = getAllowedEdits(lower);
    if (allowed === 0 || vocabulary.has(lower) || vocabulary.has(`${lower}s`) || vocabulary.has(lower.replace(/s$/, ""))) return word;
    let best = null;
    for (const candidate of candidates) {
      if (Math.abs(candidate.length - lower.length) > allowed) continue;
      const distance = levenshteinDistance(lower, candidate);
      if (distance <= allowed && (!best || distance < best.distance)) best = { word: candidate, distance };
    }
    return best ? best.word : word;
  }

  return (text) => text.replace(/[A-Za-z][A-Za-z_]*/g, (word) => fixWord(word));
}
