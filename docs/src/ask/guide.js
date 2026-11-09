import guide from "./guide.json" with { type: "json" };
import variants from "./variants.json" with { type: "json" };
import { getWords } from "./values.js";

const numberPattern = /\d/;

export const guideItems = guide.items;
export const guideMenu = guide.menu.map((id) => guideItems.find((item) => item.id === id));
export const starters = guide.starters;

export function findGuideItem(id) {
  return guideItems.find((item) => item.id === id) ?? null;
}

function matchesGroup(group, words) {
  return group.every((alternatives) => alternatives.split("|").some((word) => words.includes(word)));
}

export function matchGuideKeywords(question) {
  const hasNumber = numberPattern.test(question);
  const words = getWords(question);
  const matched = [];
  for (const item of guideItems) {
    if (hasNumber && !item.numbers) continue;
    if ((item.unless ?? []).some((word) => words.includes(word))) continue;
    if (item.all.some((group) => matchesGroup(group, words))) matched.push(item);
  }
  return matched;
}

export function getGuideTexts() {
  return guideItems.map((item) => ({ id: item.id, texts: item.questions }));
}

export function pickGuideItem(scores, candidates) {
  let best = null;
  for (const item of candidates) {
    const score = scores?.get(item.id) ?? 0;
    if (!best || score > best.score) best = { item, score };
  }
  return best;
}

export function getFunctionFollowUps(entryName) {
  return guide.functions[entryName] ?? [];
}

const unitPattern = /\d\s*([a-z]+)/gi;

function withoutUnits(question) {
  return question.replace(unitPattern, (match, unit) => match.slice(0, match.length - unit.length));
}

export function findVariant(question, entryName) {
  const forms = variants.functions[entryName];
  if (!forms) return null;
  const words = getWords(withoutUnits(question));
  for (const [form, formWords] of Object.entries(variants.words)) {
    if (!words.some((word) => formWords.includes(word))) continue;
    if (forms.forms[form]) return { form, ...forms.forms[form] };
    if (forms.refused?.[form]) return { form, refused: forms.refused[form] };
  }
  return null;
}

export function listVariants(entryName) {
  const forms = variants.functions[entryName];
  if (!forms) return [];
  return Object.entries(forms.forms).map(([form, item]) => ({ form, label: item.label }));
}

export function hasVariants(entryName) {
  return Boolean(variants.functions[entryName]);
}
