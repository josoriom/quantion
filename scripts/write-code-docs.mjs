import { readFileSync, writeFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const site = "https://phenological.github.io/quantion/";
const apiFolder = path.join(root, "docs", "public", "api");
const versionList = JSON.parse(readFileSync(path.join(apiFolder, "versions.json"), "utf8"));
const latestApi = versionList.versions.find((item) => item.version === versionList.latest).api;
const documentation = JSON.parse(readFileSync(path.join(apiFolder, `${latestApi}.json`), "utf8"));
const skippedArguments = new Set(["self", "warp", "this"]);
const missing = [];

function toCamelCase(name) {
  return name.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

function toSnakeCase(name) {
  return name.replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase();
}

function getDisplayName(entry, language) {
  return entry.kind === "function" && language === "js" ? toCamelCase(entry.name) : entry.name;
}

function getLink(entry, language) {
  return `${site}?language=${language}#${encodeURIComponent(entry.name)}`;
}

function getArguments(entry, language) {
  return (entry.arguments ?? [])
    .map((row) => ({ cell: row.languages[language], meaning: row.meaning }))
    .filter(({ cell }) => cell && !skippedArguments.has(cell.name));
}

const optionsEntries = [];
for (const entry of documentation.entries) {
  if (entry.group === "Options") optionsEntries.push(entry);
}

function findOptionsEntry(cell) {
  if (!cell) return null;
  for (const entry of optionsEntries) {
    if (new RegExp(`\\b${entry.name}\\b`).test(cell.type)) return entry;
    if (cell.type === `${toSnakeCase(entry.name)}()`) return entry;
  }
  return null;
}

function endWithPeriod(text) {
  if (text.endsWith(".")) return text;
  return `${text}.`;
}

function getOneLineDefault(defaultValue) {
  return defaultValue.replace(/\s*\n\s*/g, ", ");
}

function hasPlainDefault(defaultValue) {
  if (!defaultValue) return false;
  if (defaultValue === "default" || defaultValue === "—") return false;
  if (defaultValue.endsWith("()")) return false;
  return true;
}

function getOptionFields(optionsEntry, language) {
  const fields = [];
  for (const field of optionsEntry.fields ?? []) {
    const cell = field.languages[language];
    if (!cell) continue;
    let defaultValue = null;
    if (hasPlainDefault(cell.default)) defaultValue = getOneLineDefault(cell.default);
    fields.push({ name: cell.name, meaning: endWithPeriod(field.meaning), defaultValue, nested: findOptionsEntry(cell) });
  }
  return fields;
}

const inputEntries = [];
for (const entry of documentation.entries) {
  if (entry.kind === "type" && entry.group !== "Options" && entry.fields?.length) inputEntries.push(entry);
}

function findInputEntry(cell, language) {
  if (!cell) return null;
  for (const entry of inputEntries) {
    if (!entry.languages[language]?.example) continue;
    if (new RegExp(`\\b${entry.name}\\b`).test(cell.type)) return entry;
  }
  return null;
}

function getExampleLine(entry, language) {
  const lines = entry.languages[language].example.split("\n");
  return lines[lines.length - 1];
}

function isOptionalField(cell) {
  return /optional|^Option<|NotRequired|\?$/.test(cell.type) || cell.name.endsWith("?");
}

function getInputFields(inputEntry, language) {
  const fields = [];
  for (const field of inputEntry.fields) {
    const cell = field.languages[language];
    if (!cell) continue;
    let meaning = endWithPeriod(field.meaning);
    if (isOptionalField(cell)) meaning += " Optional.";
    fields.push({ name: cell.name, meaning });
  }
  return fields;
}

function getReturns(entry, language) {
  return entry.returns?.languages[language] ? entry.returns.meaning : null;
}

function readFile(relativePath) {
  return readFileSync(path.join(root, relativePath), "utf8").split("\n");
}

function saveFile(relativePath, lines) {
  writeFileSync(path.join(root, relativePath), lines.join("\n"));
}

function getIndent(line) {
  return line.slice(0, line.length - line.trimStart().length);
}

function formatName(name, language) {
  if (language === "python") return name;
  return `\`${name}\``;
}

function formatTypeLink(typeEntry, language) {
  const link = getLink(typeEntry, language);
  if (language === "rust") return `See [${typeEntry.name}](${link}).`;
  if (language === "js") return `See {@link ${link} | ${typeEntry.name}}.`;
  return `See ${typeEntry.name}: ${link}`;
}

function joinNames(fields, language) {
  const names = [];
  for (const field of fields) names.push(formatName(field.name, language));
  return names.join(", ");
}

function joinNamesWithDefaults(fields, language) {
  const names = [];
  for (const field of fields) {
    let name = formatName(field.name, language);
    if (field.defaultValue) name += ` = ${field.defaultValue}`;
    names.push(name);
  }
  return names.join(", ");
}

function getArgumentText(cell, meaning, language) {
  const optionsEntry = findOptionsEntry(cell);
  if (optionsEntry) {
    const names = joinNamesWithDefaults(getOptionFields(optionsEntry, language), language);
    return { text: `Optional settings: ${names}. ${formatTypeLink(optionsEntry, language)}`, example: null };
  }
  const inputEntry = findInputEntry(cell, language);
  if (inputEntry) {
    const names = joinNames(getInputFields(inputEntry, language), language);
    return {
      text: `${meaning} Each ${inputEntry.name} has ${names}. ${formatTypeLink(inputEntry, language)}`,
      example: getExampleLine(inputEntry, language),
    };
  }
  return { text: meaning, example: null };
}

function escapeRustText(text) {
  return text.replace(/\[(?=[^\]]*\](?!\())/g, "\\[").replace(/(?<=\\\[[^\]]*)\]/g, "\\]");
}

function getFieldLines(entry, language) {
  const lines = [];
  for (const field of entry.fields ?? []) {
    const cell = field.languages[language];
    if (!cell) continue;
    let meaning = endWithPeriod(field.meaning);
    if (language === "rust") meaning = escapeRustText(meaning);
    let line = `${formatName(cell.name, language)}: ${meaning}`;
    if (entry.group === "Options" && hasPlainDefault(cell.default)) line += ` Default ${formatName(getOneLineDefault(cell.default), language)}.`;
    if (entry.group !== "Options" && isOptionalField(cell)) line += " Optional.";
    const nested = findOptionsEntry(cell);
    if (nested) line += ` ${formatTypeLink(nested, language)}`;
    lines.push(line);
  }
  return lines;
}

function getTypeExample(entry, language) {
  const cell = entry.languages[language];
  if (!cell?.example) return null;
  return getExampleLine(entry, language);
}

function getRustBlock(entry) {
  const lines = [`**${entry.name}** · ${entry.group}`, "", entry.summary];
  const warning = entry.languages.rust?.warning ?? entry.warning;
  if (warning) lines.push("", `**Warning:** ${escapeRustText(warning)}`);
  const argumentsList = getArguments(entry, "rust");
  if (argumentsList.length) {
    lines.push("");
    for (const { cell, meaning } of argumentsList) {
      const argument = getArgumentText(cell, meaning, "rust");
      lines.push(`- \`${cell.name}\`: ${argument.text}`);
      if (argument.example) lines.push(`  Example: \`${argument.example}\``);
    }
  }
  if (entry.kind === "type") {
    const fieldLines = getFieldLines(entry, "rust");
    if (fieldLines.length) {
      lines.push("", "Fields:");
      for (const line of fieldLines) lines.push(`- ${line}`);
    }
    const example = getTypeExample(entry, "rust");
    if (example) lines.push("", "Example:", "```ignore", example, "```");
  }
  const returns = getReturns(entry, "rust");
  if (returns) lines.push("", `Returns: ${returns}`);
  lines.push("", `[Documentation](${getLink(entry, "rust")})`);
  return lines;
}

function writeRust(entry, relativePaths, pattern) {
  for (const relativePath of relativePaths) {
    const lines = readFile(relativePath);
    const index = lines.findIndex((line) => pattern.test(line));
    if (index < 0) continue;
    let start = index;
    while (start > 0 && /^\s*(#\[|\/\/\/)/.test(lines[start - 1])) start -= 1;
    const kept = lines.slice(start, index).filter((line) => !/^\s*\/\/\//.test(line));
    const indent = getIndent(lines[index]);
    const block = getRustBlock(entry).map((line) => `${indent}///${line ? ` ${line}` : ""}`);
    lines.splice(start, index - start, ...block, ...kept);
    saveFile(relativePath, lines);
    return true;
  }
  return false;
}

function getPythonBlock(entry) {
  const lines = [`${entry.name} · ${entry.group}`, "", entry.summary];
  if (entry.warning) lines.push("", "Warning:", `    ${entry.warning}`);
  const argumentsList = getArguments(entry, "python");
  if (argumentsList.length) {
    lines.push("", "Args:");
    for (const { cell, meaning } of argumentsList) {
      const argument = getArgumentText(cell, meaning, "python");
      lines.push(`    ${cell.name}: ${argument.text}`);
      if (argument.example) lines.push(`        Example: ${argument.example}`);
    }
  }
  if (entry.kind === "type") {
    const fieldLines = getFieldLines(entry, "python");
    if (fieldLines.length) {
      lines.push("", "Fields:");
      for (const line of fieldLines) lines.push(`    ${line}`);
    }
    const example = getTypeExample(entry, "python");
    if (example) lines.push("", "Example:", `    ${example}`);
  }
  const returns = getReturns(entry, "python");
  if (returns) lines.push("", "Returns:", `    ${returns}`);
  lines.push("", `Documentation: ${getLink(entry, "python")}`);
  return lines;
}

function removePythonDocstring(lines, index) {
  if (!/^\s*"""/.test(lines[index] ?? "")) return;
  const single = lines[index].trim().length > 3 && lines[index].trim().endsWith('"""');
  let end = index;
  if (!single) {
    end += 1;
    while (end < lines.length && !lines[end].includes('"""')) end += 1;
  }
  lines.splice(index, end - index + 1);
}

function writePythonBody(entry, relativePath, pattern) {
  const lines = readFile(relativePath);
  const index = lines.findIndex((line) => pattern.test(line));
  if (index < 0) return false;
  let end = index;
  while (!/:\s*$/.test(lines[end])) end += 1;
  const indent = `${getIndent(lines[index])}    `;
  removePythonDocstring(lines, end + 1);
  const block = getPythonBlock(entry);
  const docstring = [`${indent}"""${block[0]}`, ...block.slice(1).map((line) => (line ? `${indent}${line}` : "")), `${indent}"""`];
  lines.splice(end + 1, 0, ...docstring);
  saveFile(relativePath, lines);
  return true;
}

function writePythonAssignedDoc(entry, relativePath, name) {
  const lines = readFile(relativePath);
  const index = lines.findIndex((line) => line.startsWith(`${name} = TypedDict(`));
  if (index < 0) return false;
  let end = index;
  while (!/\)\s*$/.test(lines[end])) end += 1;
  if ((lines[end + 1] ?? "").startsWith(`${name}.__doc__ = `)) {
    let docEnd = end + 1;
    while (!lines[docEnd].trimEnd().endsWith('"""') || docEnd === end + 1 && lines[docEnd].trim() === `${name}.__doc__ = """`) docEnd += 1;
    lines.splice(end + 1, docEnd - end);
  }
  const block = getPythonBlock(entry);
  lines.splice(end + 1, 0, `${name}.__doc__ = """${block[0]}`, ...block.slice(1), '"""');
  saveFile(relativePath, lines);
  return true;
}

function getJsBlock(entry, indent) {
  const lines = [`${getDisplayName(entry, "js")} · ${entry.group}`, "", entry.summary];
  if (entry.warning) lines.push("", `**Warning:** ${entry.warning}`);
  const argumentsList = getArguments(entry, "js");
  if (argumentsList.length) {
    lines.push("");
    for (const { cell, meaning } of argumentsList) {
      const argument = getArgumentText(cell, meaning, "js");
      lines.push(`@param ${cell.name} - ${argument.text}`);
      if (argument.example) lines.push(`  Example: \`${argument.example}\``);
    }
  }
  if (entry.kind === "type") {
    const fieldLines = getFieldLines(entry, "js");
    if (fieldLines.length) {
      lines.push("", "Fields:");
      for (const line of fieldLines) lines.push(`- ${line}`);
    }
    const example = getTypeExample(entry, "js");
    if (example) lines.push("", "@example", "```ts", example, "```");
  }
  const returns = getReturns(entry, "js");
  if (returns) lines.push(`@returns ${returns}`);
  lines.push(`@see {@link ${getLink(entry, "js")} | Documentation}`);
  const output = [`${indent}/**`];
  for (const line of lines) {
    if (line) output.push(`${indent} * ${line}`);
    else output.push(`${indent} *`);
  }
  output.push(`${indent} */`);
  return output;
}

function writeJs(entry, relativePaths, pattern) {
  for (const relativePath of relativePaths) {
    const lines = readFile(relativePath);
    const index = lines.findIndex((line) => pattern.test(line));
    if (index < 0) continue;
    let start = index;
    if (/^\s*\*\/\s*$/.test(lines[index - 1] ?? "")) {
      start = index - 1;
      while (start > 0 && !/^\s*\/\*\*/.test(lines[start])) start -= 1;
    } else if (/^\s*\/\*\*.*\*\/\s*$/.test(lines[index - 1] ?? "")) {
      start = index - 1;
    }
    lines.splice(start, index - start, ...getJsBlock(entry, getIndent(lines[index])));
    saveFile(relativePath, lines);
    return true;
  }
  return false;
}

function escapeRd(text) {
  const escaped = text
    .replace(/\\/g, "\\\\")
    .replace(/%/g, "\\%")
    .replace(/\{/g, "\\{")
    .replace(/\}/g, "\\}")
    .replace(/–/g, "-")
    .replace(/—/g, "-")
    .replace(/…/g, "...")
    .replace(/[‘’]/g, "'")
    .replace(/[“”]/g, '"')
    .replace(/±/g, "+/-")
    .replace(/×/g, "x");
  if (/[^\x00-\x7F]/.test(escaped)) throw new Error(`R help pages must be ASCII: ${escaped}`);
  return escaped;
}

function findRdBlock(text, tag) {
  const start = text.indexOf(`\\${tag}{`);
  if (start < 0) return null;
  let depth = 0;
  for (let index = start + tag.length + 1; index < text.length; index += 1) {
    const character = text[index];
    if (character === "\\") {
      index += 1;
      continue;
    }
    if (character === "{") depth += 1;
    if (character === "}") {
      depth -= 1;
      if (depth === 0) return { start, end: index + 1 };
    }
  }
  return null;
}

function replaceRdBlock(text, tag, body) {
  const block = findRdBlock(text, tag);
  if (!block) return text;
  return `${text.slice(0, block.start)}\\${tag}{\n${body}\n}${text.slice(block.end)}`;
}

function getRTypeEntry(type) {
  for (const entry of documentation.entries) {
    if (entry.kind !== "type" || !entry.fields?.length) continue;
    if (new RegExp(`\\b${entry.name}\\b`).test(type)) return entry;
  }
  return null;
}

function getRFieldNames(entry) {
  const names = [];
  for (const field of entry.fields) {
    const cell = field.languages.r;
    if (cell) names.push(`\\code{${cell.name}}`);
  }
  return names.join(", ");
}

function hasRFunction(entry) {
  return entry.languages.r.signature.startsWith(`${toSnakeCase(entry.name)}(`);
}

function getROptionsArgument(cell, optionsEntry) {
  const names = [];
  for (const field of getOptionFields(optionsEntry, "r")) {
    if (field.defaultValue) names.push(`\\code{${field.name} = ${escapeRd(field.defaultValue)}}`);
    else names.push(`\\code{${field.name}}`);
  }
  return [
    `  \\item{${cell.name}}{Settings from \\code{\\link{${toSnakeCase(optionsEntry.name)}}()}: ${names.join(", ")}.`,
    `    \\href{${getLink(optionsEntry, "r")}}{${optionsEntry.name} documentation}`,
    "  }",
  ];
}

function getRArgument(row, cell) {
  const optionsEntry = findOptionsEntry(cell);
  if (optionsEntry) return getROptionsArgument(cell, optionsEntry);
  const text = `${escapeRd(endWithPeriod(row.meaning))} Type: \\code{${escapeRd(cell.type)}}.`;
  const inputEntry = findInputEntry(cell, "r");
  if (!inputEntry) return [`  \\item{${cell.name}}{${text}}`];
  return [
    `  \\item{${cell.name}}{${text} Each ${inputEntry.name} has ${getRFieldNames(inputEntry)}.`,
    `    Example: \\code{${escapeRd(getExampleLine(inputEntry, "r"))}}`,
    `    \\href{${getLink(inputEntry, "r")}}{${inputEntry.name} documentation}`,
    "  }",
  ];
}

function getRValue(entry) {
  const type = entry.returns?.languages.r;
  if (!type) return null;
  let text = `${escapeRd(endWithPeriod(entry.returns.meaning))} Type: \\code{${escapeRd(type)}}.`;
  const typeEntry = getRTypeEntry(type);
  if (typeEntry) {
    let label = "Fields";
    if (type.includes("data.frame")) label = "Columns";
    text += ` ${label}: ${getRFieldNames(typeEntry)}.`;
  }
  return text;
}

function getRExample(example, runs) {
  const code = example.replace(/\\/g, "\\\\").replace(/%/g, "\\%");
  if (!runs) return `\\dontrun{\n${code}\n}`;
  const indented = [];
  for (const line of code.split("\n")) {
    if (line) indented.push(`  ${line}`);
    else indented.push("");
  }
  return `\\donttest{\nif (curl::has_internet()) {\n${indented.join("\n")}\n}\n}`;
}

function writeRFunctionPage(entry, text) {
  text = replaceRdBlock(text, "description", escapeRd(endWithPeriod(entry.summary)));
  const argumentLines = [];
  for (const row of entry.arguments ?? []) {
    const cell = row.languages.r;
    if (cell) argumentLines.push(...getRArgument(row, cell));
  }
  if (argumentLines.length) text = replaceRdBlock(text, "arguments", argumentLines.join("\n"));
  const value = getRValue(entry);
  if (value) text = replaceRdBlock(text, "value", value);
  const example = entry.languages.r.example;
  if (example) text = replaceRdBlock(text, "examples", getRExample(example, entry.languages.r.exampleRuns !== false));
  return text;
}

function writeROptionsPage(entry, text) {
  text = replaceRdBlock(text, "description", escapeRd(endWithPeriod(entry.summary)));
  const argumentLines = [];
  for (const field of entry.fields ?? []) {
    const cell = field.languages.r;
    if (!cell) continue;
    let line = `  \\item{${cell.name}}{${escapeRd(endWithPeriod(field.meaning))}`;
    const nested = findOptionsEntry(cell);
    if (nested) line += hasRFunction(nested) ? ` See \\code{\\link{${toSnakeCase(nested.name)}}()}.` : ` See \\href{${getLink(nested, "r")}}{${nested.name}}.`;
    argumentLines.push(`${line}}`);
  }
  if (argumentLines.length) text = replaceRdBlock(text, "arguments", argumentLines.join("\n"));
  return text;
}

function writeR(entry, relativePath) {
  if (!existsSync(path.join(root, relativePath))) return false;
  let text = readFileSync(path.join(root, relativePath), "utf8");
  if (entry.kind === "function") text = writeRFunctionPage(entry, text);
  if (entry.group === "Options") text = writeROptionsPage(entry, text);
  let section = `\\section{Documentation}{\n\\url{${getLink(entry, "r")}}\n}\n`;
  if (entry.warning) section = `\\section{Warning}{\n${escapeRd(entry.warning)}\n}\n${section}`;
  text = text.replace(/\\section\{Warning\}\{\n[^\n]*\n\}\n/, "");
  text = text.replace(/\\section\{Documentation\}\{\n[^\n]*\n\}\n/, "");
  text = text.includes("\\examples{") ? text.replace("\\examples{", `${section}\\examples{`) : `${text.trimEnd()}\n${section}`;
  writeFileSync(path.join(root, relativePath), text);
  return true;
}

const rustFunctionFiles = ["core/src/files.rs", "core/src/traces.rs", "core/src/targets.rs", "core/src/features.rs", "core/src/models.rs"];
const rustTypeFiles = [
  "core/src/options.rs",
  "core/src/error.rs",
  "core/src/sample_file.rs",
  "core/src/files.rs",
  "core/src/utilities/structs.rs",
  "core/src/utilities/find_features.rs",
  "core/src/utilities/find_noise_level.rs",
  "core/src/utilities/fit_peak.rs",
  "core/src/utilities/ion_image.rs",
  "core/src/utilities/warp_model.rs",
  "core/src/utilities/calculate_eic.rs",
];
const pythonMethodFiles = { predict_rts: ["wrappers/python/quantion/warp_model.py"], calibrate: ["wrappers/python/quantion/warp_model.py"], dispose: ["wrappers/python/quantion/file.py", "wrappers/python/quantion/model.py", "wrappers/python/quantion/warp_model.py"] };
const jsMethodFiles = { predict_rts: ["wrappers/js/src/utilities/warpModel.ts"], calibrate: ["wrappers/js/src/utilities/warpModel.ts"], dispose: ["wrappers/js/src/utilities/sampleFile.ts", "wrappers/js/src/utilities/model.ts", "wrappers/js/src/utilities/warpModel.ts"] };
const jsTypeFiles = ["wrappers/js/src/types/types.ts", "wrappers/js/src/utilities/sampleFile.ts", "wrappers/js/src/utilities/errors.ts"];
const pythonTypeFiles = ["wrappers/python/quantion/_types.py", "wrappers/python/quantion/_errors.py", "wrappers/python/quantion/file.py"];

for (const entry of documentation.entries) {
  const isFunction = entry.kind === "function";
  if (entry.languages.rust) {
    const name = entry.name;
    const written = isFunction
      ? writeRust(entry, rustFunctionFiles, new RegExp(`^\\s*pub fn ${name}\\(`))
      : writeRust(entry, rustTypeFiles, new RegExp(`^\\s*pub (struct|enum) ${name}\\b`));
    if (!written) missing.push(`rust ${name}`);
  }
  if (entry.languages.python) {
    let written = false;
    if (pythonMethodFiles[entry.name]) {
      written = pythonMethodFiles[entry.name].map((file) => writePythonBody(entry, file, new RegExp(`^\\s+def ${entry.name}\\(`))).some(Boolean);
    } else if (isFunction) {
      written = writePythonBody(entry, "wrappers/python/quantion/api.py", new RegExp(`^def ${entry.name}\\(`));
    } else {
      written = pythonTypeFiles.some((file) => writePythonBody(entry, file, new RegExp(`^class ${entry.name}\\b`)) || writePythonAssignedDoc(entry, file, entry.name));
    }
    if (!written) missing.push(`python ${entry.name}`);
  }
  if (entry.languages.js) {
    const name = getDisplayName(entry, "js");
    let written = false;
    if (jsMethodFiles[entry.name]) {
      written = jsMethodFiles[entry.name].map((file) => writeJs(entry, [file], new RegExp(`^\\s+(async )?${name}\\(`))).some(Boolean);
    } else if (isFunction) {
      written = writeJs(entry, ["wrappers/js/src/utilities/api.ts"], new RegExp(`^export (async )?function ${name}\\(`));
    } else {
      written = writeJs(entry, jsTypeFiles, new RegExp(`^export (interface|const|class|type) ${name}\\b`));
    }
    if (!written) missing.push(`js ${name}`);
  }
  if (entry.languages.r) {
    const rdName = isFunction ? entry.name : entry.name === "Header" ? "header" : toSnakeCase(entry.name);
    if (!writeR(entry, `wrappers/r/man/${rdName}.Rd`) && (isFunction || entry.name.endsWith("Options") || entry.name === "Header")) {
      missing.push(`r ${rdName}`);
    }
  }
}

console.log(missing.length ? `not found:\n  ${missing.join("\n  ")}` : "every item has its documentation");
