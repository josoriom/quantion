import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { createAsker, createEmbedder, writeAnswer } from "../src/ask/ask.js";
import { models } from "../src/ask/models.js";
import { buildTemplateSnippet } from "../src/ask/snippet.js";

const folder = path.dirname(fileURLToPath(import.meta.url));
const publicFolder = path.join(folder, "..", "public");
const snippetLanguages = ["python", "js", "r", "rust"];
const checkFolder = mkdtempSync(path.join(tmpdir(), "ask-eval-"));
const extensions = { python: "py", js: "mjs", r: "R", rust: "rs" };
const syntaxCheckers = {
  python: (file) => ["python3", ["-I", "-c", "import ast, sys; ast.parse(open(sys.argv[1]).read())", file]],
  js: (file) => ["node", ["--check", file]],
  r: (file) => ["Rscript", ["-e", `invisible(parse(file = "${file}"))`]],
};
const routes = ["usage", "definition", "placement", "review", "guide", "list", "command", "refused"];
const confidences = ["high", "medium", "low"];
const splits = ["dev", "test", "all"];

function readJson(file) {
  return JSON.parse(readFileSync(file, "utf8"));
}

function readSet(name) {
  return readJson(path.join(folder, name));
}

function findApi(list) {
  for (const item of list.versions) {
    if (item.version === list.latest) return item.api;
  }
  throw new Error(`no api file for ${list.latest}`);
}

function checkSyntax(code, language) {
  const checker = syntaxCheckers[language];
  if (!checker) return true;
  const file = path.join(checkFolder, `snippet.${extensions[language]}`);
  writeFileSync(file, code);
  const [command, commandArguments] = checker(file);
  return spawnSync(command, commandArguments).status === 0;
}

function sameValues(left, right) {
  const leftKeys = Object.keys(left).sort();
  const rightKeys = Object.keys(right).sort();
  if (leftKeys.join() !== rightKeys.join()) return false;
  return leftKeys.every((key) => left[key] === right[key]);
}

function withoutFunction(values) {
  const { function: _, ...rest } = values;
  return rest;
}

function mentions(text, name) {
  return new RegExp(`\`${name}\``).test(text);
}

function pad(text, width) {
  return String(text).padStart(width);
}

function countBySplit(items, isRight) {
  return splits.map((split) => {
    const chosen = items.filter((item) => split === "all" || item.split === split);
    return pad(`${chosen.filter(isRight).length}/${chosen.length}`, 10);
  });
}

function printTable(title, rows) {
  console.log(`\n${title.padEnd(12)}${splits.map((split) => pad(split, 10)).join("")}`);
  for (const [label, cells] of rows) console.log(`${label.padEnd(12)}${cells.join("")}`);
}

const api = findApi(readJson(path.join(publicFolder, "api", "versions.json")));
const document = readJson(path.join(publicFolder, "api", `${api}.json`));
const index = readJson(path.join(publicFolder, "ask", `${api}.json`));
const ask = createAsker({ embed: await createEmbedder({ modelFolder: `${path.join(publicFolder, models.folder)}/` }), index, document });

const records = [];

function record(set, item, expectedRoute, result, right, text) {
  const route = result.reason.route;
  records.push({ set, split: item.split, expectedRoute, route, confidence: result.reason.confidence, right, text });
  if (!right) console.log(`-- [${item.split}] ${set}: ${text}`);
}

function describeRoute(result) {
  if (result.reason.route === "guide") return `guide:${result.guide.id}`;
  if (result.reason.route === "usage") return `usage:${result.entries[0]?.name}${result.variant ? `/${result.variant.form}` : ""}`;
  return result.reason.route;
}

async function makePrevious(question) {
  if (!question) return null;
  const first = await ask(question, "python");
  return { question, questionVector: first.questionVector, functionName: first.entries[0]?.name, option: first.option ?? null };
}

const usageRows = [];
for (const item of readSet("ask-questions.json")) {
  const result = await ask(item.question, "python");
  const isGuide = result.reason.route === "guide";
  const names = result.entries.map((entry) => entry.name);
  const top1 = names[0] === item.function;
  const answer = result.values ? writeAnswer(result, document, "python") : "";
  const allValues = { ...withoutFunction(result.values ?? {}), ...Object.assign({}, ...Object.values(result.templateValues ?? {})) };
  const extraOption = item.options.length === 0 && /set its option/.test(answer);
  const rightOptions = isGuide ? item.options.length === 0 : item.options.every((option) => mentions(answer, option)) && !extraOption;
  const rightValues = isGuide ? Object.keys(item.values).length === 0 : sameValues(allValues, item.values);
  const rightVariant = !item.variant || result.variant?.form === item.variant;
  const row = { split: item.split, refused: Boolean(result.unknown), top1, top3: names.includes(item.function), options: rightOptions, values: rightValues, variant: rightVariant, confidence: result.reason.confidence };
  row.allRight = top1 && rightOptions && rightValues && rightVariant;
  const marks = [];
  for (const language of snippetLanguages) {
    const snippet = result.values ? buildTemplateSnippet(result.values, result.templateValues, document, language) : { valid: false, reason: result.unknown ? "refused" : "no values" };
    const rightFunction = snippet.valid && snippet.function === item.function;
    const placedRight = snippet.valid && sameValues(snippet.placed, item.values);
    const syntaxOk = snippet.valid && checkSyntax(snippet.code, language);
    row[language] = rightFunction && placedRight && syntaxOk;
    marks.push(`${language}:${snippet.valid ? `${rightFunction ? "F" : "f"}${placedRight ? "V" : "v"}${syntaxOk ? "S" : "s"}` : snippet.reason}`);
  }
  usageRows.push(row);
  records.push({ set: "usage", split: item.split, expectedRoute: "usage", route: result.reason.route, confidence: result.reason.confidence, right: row.allRight, top1, text: item.question });
  console.log(`${row.allRight ? "ok " : "-- "}[${item.split}] ${item.question}\n     ${describeRoute(result)} (${result.reason.confidence})  top3: ${names.join(", ")}\n     answer: ${answer}\n     snippet: ${marks.join("  ")}`);
}

const measures = ["top1", "top3", "options", "values", "variant", "allRight", ...snippetLanguages];
printTable("usage", measures.map((measure) => [measure, countBySplit(usageRows, (row) => row[measure])]));
console.log("python/js/r/rust rows: snippet fully right (rust has no syntax check)");

function getExplained(explanation) {
  if (explanation.type === "entry") return explanation.entry.name;
  if (explanation.type === "group") return explanation.group.name;
  return `${explanation.term}@${explanation.groups[0].matches[0].entry.name}`;
}

for (const item of readSet("ask-off-topic.json")) {
  const result = await ask(item.question, "python");
  record("offTopic", item, "refused", result, Boolean(result.unknown), `${item.question} -> ${describeRoute(result)}`);
}
for (const item of readSet("ask-definitions.json")) {
  const result = await ask(item.question, "python", null, item.page);
  const got = result.explanation ? getExplained(result.explanation) : describeRoute(result);
  record("define", item, "definition", result, got === item.expect, `${item.question} -> ${got}`);
}
for (const item of readSet("ask-reviews.json")) {
  const result = await ask(item.code, "js");
  const fixes = Object.fromEntries((result.review?.fixes ?? []).map((fix) => [fix.from, fix.to]));
  const right = Boolean(result.review) && JSON.stringify(fixes) === JSON.stringify(item.fixes) && (result.review.notes.length > 0) === Boolean(item.notes);
  record("review", item, "review", result, right, `${item.code} -> ${JSON.stringify(fixes)} notes:${result.review?.notes.length ?? "none"}`);
}
for (const item of readSet("ask-placements.json")) {
  const result = await ask(item.question, "python", await makePrevious(item.previous));
  const got = result.placement ? `${result.placement.item.languages.python.name}@${result.placement.owner.name}` : describeRoute(result);
  record("place", item, "placement", result, got === item.expect, `${item.question} -> ${got}`);
}
for (const item of readSet("ask-lists.json")) {
  const result = await ask(item.question, "python");
  const got = result.list ? `${result.list.option}:${result.list.entries.map((entry) => entry.name).join(",")}` : describeRoute(result);
  record("list", item, "list", result, got === item.expect, `${item.question} -> ${got}`);
}
for (const item of readSet("ask-guide.json")) {
  const result = await ask(item.question, "python");
  const got = result.guide ? result.guide.id : describeRoute(result);
  record("guide", item, "guide", result, got === item.expect, `${item.question} -> ${got}`);
}

function describeCommand(result) {
  const command = result.command;
  if (!command) return describeRoute(result);
  if (command.command === "docs") return `docs:${command.target?.entryName ?? "none"}`;
  if (command.command === "example") {
    if (!command.entry) return "example:failure";
    if (command.failure) return `example:${command.entry.name}:failure`;
    return `example:${command.entry.name}:${Object.entries(withoutFunction(command.values)).map(([key, value]) => `${key}=${value}`).join(",")}`;
  }
  if (command.command === "fix") {
    if (command.failure) return "fix:failure";
    return `fix:${command.review.fixes.map((fix) => `${fix.from}=${fix.to}`).join(",")}`;
  }
  return command.command;
}

for (const item of readSet("ask-commands.json")) {
  const result = await ask(item.question, "js");
  const got = describeCommand(result);
  record("command", item, "command", result, got === item.expect, `${item.question} -> ${got}`);
}

for (const item of readSet("ask-followups.json")) {
  const result = await ask(item.question, "python", await makePrevious(item.previous));
  let right = result.reason.route === item.route && (item.route !== "usage" || result.reason.confidence !== "low");
  if (right && item.function) right = result.entries[0]?.name === item.function;
  if (right && item.variant) right = result.variant?.form === item.variant;
  if (right && item.values) right = sameValues(withoutFunction(result.values ?? {}), item.values);
  record("followUp", item, item.route, result, right, `${item.previous} => ${item.question} -> ${describeRoute(result)} ${JSON.stringify(withoutFunction(result.values ?? {}))}`);
}

const sets = ["offTopic", "define", "review", "place", "guide", "list", "command", "followUp"];
printTable("set", sets.map((set) => [set, countBySplit(records.filter((row) => row.set === set), (row) => row.right)]));
console.log(`${"refused".padEnd(12)}${countBySplit(usageRows, (row) => row.refused).join("")}   (on-topic usage questions refused)`);

console.log(`\n${"expected \\ chosen".padEnd(20)}${routes.map((route) => pad(route, 11)).join("")}`);
for (const expected of routes) {
  const rows = records.filter((row) => row.expectedRoute === expected);
  if (rows.length === 0) continue;
  console.log(`${expected.padEnd(20)}${routes.map((route) => pad(rows.filter((row) => row.route === route).length || ".", 11)).join("")}`);
}

console.log(`\n${"confidence".padEnd(12)}${pad("route ok", 12)}${pad("answer ok", 12)}${pad("function ok", 13)}`);
for (const confidence of confidences) {
  const rows = records.filter((row) => row.confidence === confidence);
  const usage = rows.filter((row) => row.set === "usage" && row.route === "usage");
  const routeOk = rows.filter((row) => row.route === row.expectedRoute).length;
  const answerOk = rows.filter((row) => row.right).length;
  console.log(`${confidence.padEnd(12)}${pad(`${routeOk}/${rows.length}`, 12)}${pad(`${answerOk}/${rows.length}`, 12)}${pad(`${usage.filter((row) => row.top1).length}/${usage.length}`, 13)}`);
}
