import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { createEmbedder, describeEntryTexts, encodeVectors } from "../src/ask/ask.js";
import { getGuideTexts } from "../src/ask/guide.js";
import { models } from "../src/ask/models.js";

const publicFolder = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "public");

function readJson(file) {
  return JSON.parse(readFileSync(path.join(publicFolder, file), "utf8"));
}

function findApi(list) {
  for (const item of list.versions) {
    if (item.version === list.latest) return item.api;
  }
  throw new Error(`no api file for ${list.latest}`);
}

const api = process.argv[2] ?? findApi(readJson("api/versions.json"));
const document = readJson(`api/${api}.json`);
const embed = await createEmbedder({ modelFolder: `${path.join(publicFolder, models.folder)}/` });
const entries = [];

let count = 0;
for (const entry of document.entries) {
  const vectors = [];
  for (const text of describeEntryTexts(entry, document)) vectors.push(await embed(text));
  entries.push({ name: entry.name, vectors: encodeVectors(vectors) });
  count += vectors.length;
}

const anchorTexts = JSON.parse(readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "ask-anchors.json"), "utf8"));
const anchorVectors = [];
for (const text of anchorTexts) anchorVectors.push(await embed(text));
const anchors = encodeVectors(anchorVectors);

const guide = [];
for (const item of getGuideTexts()) {
  const vectors = [];
  for (const text of item.texts) vectors.push(await embed(text));
  guide.push({ id: item.id, vectors: encodeVectors(vectors) });
}

mkdirSync(path.join(publicFolder, "ask"), { recursive: true });
writeFileSync(path.join(publicFolder, "ask", `${api}.json`), `${JSON.stringify({ api, model: models.embedder.name, dimension: 384, entries, anchors, guide })}\n`);
console.log(`ask/${api}.json: ${entries.length} entries, ${count} vectors, ${guide.length} guide items`);
