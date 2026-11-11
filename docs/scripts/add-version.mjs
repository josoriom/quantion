import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const folder = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "public", "api");
const input = process.argv.slice(2);
const newVersion = input[0]?.replace(/^v/, "");
const hasNewApi = input.includes("--new-api");
const messageIndex = input.indexOf("--message");
const givenMessage = getGivenMessage();

if (!newVersion || !/^\d+\.\d+\.\d+$/.test(newVersion) || (messageIndex >= 0 && !givenMessage)) {
  console.error('usage: npm run add-version -- 0.2.1 [--new-api] [--message "commit message"]');
  process.exit(1);
}

function getGivenMessage() {
  if (messageIndex < 0) return null;
  return input[messageIndex + 1];
}

function findVersion(versions, version) {
  for (const item of versions) {
    if (item.version === version) return item;
  }
  return null;
}

function getLastCommitMessage() {
  return execFileSync("git", ["log", "-1", "--format=%s"], { encoding: "utf8" }).trim();
}

function getToday() {
  return new Date().toISOString().slice(0, 10);
}

const listPath = path.join(folder, "versions.json");
const list = JSON.parse(readFileSync(listPath, "utf8"));
const release = { date: getToday(), message: givenMessage ?? getLastCommitMessage() };
const listed = findVersion(list.versions, newVersion);

function saveList(versions) {
  writeFileSync(listPath, `${JSON.stringify({ ...list, versions }, null, 2)}\n`);
}

if (listed) {
  if (listed.date) {
    console.error(`${newVersion} is already released on ${listed.date}`);
    process.exit(1);
  }
  Object.assign(listed, release);
  saveList(list.versions);
  console.log(`released ${newVersion} on ${release.date}: ${release.message}`);
  process.exit(0);
}

const latestApi = findVersion(list.versions, list.latest).api;
let api = latestApi;

if (hasNewApi) {
  api = newVersion;
  const newPath = path.join(folder, `${api}.json`);
  if (existsSync(newPath)) {
    console.error(`${api}.json already exists`);
    process.exit(1);
  }
  writeFileSync(newPath, readFileSync(path.join(folder, `${latestApi}.json`)));
}

list.latest = newVersion;
saveList([{ version: newVersion, api, ...release }, ...list.versions]);

if (hasNewApi) {
  console.log(`added ${newVersion} with its own API file public/api/${api}.json; edit it to match the new API`);
} else {
  console.log(`added ${newVersion}, sharing public/api/${api}.json`);
}
console.log(`released on ${release.date}: ${release.message}`);
