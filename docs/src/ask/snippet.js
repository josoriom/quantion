import { getDisplayName } from "../names.js";

const integerTypes = new Set(["usize", "u8", "u16", "u32", "u64", "i32", "i64", "isize"]);
const numberTypes = new Set(["f64", "f32"]);
const textTypes = new Set(["String", "&str", "PathBuf"]);

function findByName(document, name) {
  for (const entry of document.entries) {
    if (entry.name === name) return entry;
  }
  return null;
}

function unwrapOption(type) {
  const match = /^Option<(.+)>$/.exec(type);
  if (match) return { type: match[1], isOptional: true };
  return { type, isOptional: false };
}

export function getKind(rustType, document) {
  const { type, isOptional } = unwrapOption(rustType);
  if (numberTypes.has(type)) return { kind: "number", isOptional };
  if (integerTypes.has(type)) return { kind: "integer", isOptional };
  if (type === "bool") return { kind: "boolean", isOptional };
  if (type === "Position") return { kind: "number", isOptional: true, isPosition: true };
  if (textTypes.has(type)) return { kind: "text", isOptional };
  const entry = findByName(document, type);
  if (entry?.group === "Enums") return { kind: "enum", isOptional, entry };
  if (entry?.group === "Options") return { kind: "options", isOptional, entry };
  return { kind: "other", isOptional };
}

export function getOptionsType(entry, document) {
  for (const argument of entry.arguments ?? []) {
    const type = findByName(document, argument.languages.rust?.type ?? "");
    if (type?.group === "Options") return type;
  }
  return null;
}

function findOptionPath(optionsType, key, document) {
  for (const field of optionsType.fields ?? []) {
    if (field.languages.python.name === key) return [field];
  }
  for (const field of optionsType.fields ?? []) {
    const kind = getKind(field.languages.rust?.type ?? "", document);
    if (kind.kind !== "options") continue;
    for (const inner of kind.entry.fields ?? []) {
      if (inner.languages.python.name === key) return [field, inner];
    }
  }
  return null;
}

function checkValue(value, kind) {
  if (kind.kind === "number") return typeof value === "number" && Number.isFinite(value);
  if (kind.kind === "integer") return Number.isInteger(value) && value >= 0;
  if (kind.kind === "boolean") return typeof value === "boolean";
  if (kind.kind === "text") return typeof value === "string" && value.length > 0;
  if (kind.kind === "enum") {
    for (const field of kind.entry.fields ?? []) {
      if (field.languages.python.name === String(value).toUpperCase()) return true;
    }
  }
  return false;
}

function normalizeValue(value, kind) {
  if (kind.kind === "enum") return String(value).toUpperCase();
  return value;
}

export function validateValues(values, document) {
  if (!values || typeof values !== "object" || Array.isArray(values)) return { valid: false, reason: "no values" };
  const entry = findByName(document, values.function);
  if (!entry || entry.kind !== "function") return { valid: false, reason: `unknown function ${values.function}` };
  const optionsType = getOptionsType(entry, document);
  const argumentValues = [];
  const optionValues = [];
  for (const [key, value] of Object.entries(values)) {
    if (key === "function") continue;
    const argument = (entry.arguments ?? []).find((item) => item.languages.python.name === key);
    if (argument) {
      const kind = getKind(argument.languages.rust?.type ?? "", document);
      if (!checkValue(value, kind)) return { valid: false, reason: `bad value for ${key}` };
      argumentValues.push({ argument, kind, value: normalizeValue(value, kind) });
      continue;
    }
    const path = optionsType && findOptionPath(optionsType, key, document);
    if (!path) return { valid: false, reason: `unknown option ${key}` };
    const kind = getKind(path[path.length - 1].languages.rust?.type ?? "", document);
    if (!checkValue(value, kind)) return { valid: false, reason: `bad value for ${key}` };
    optionValues.push({ path, kind, value: normalizeValue(value, kind) });
  }
  return { valid: true, entry, optionsType, argumentValues, optionValues };
}

function formatNumber(value, language, kind) {
  if (kind.kind === "integer") return language === "r" ? `${value}L` : String(value);
  const text = String(value);
  if (language === "rust" && !/[.e]/.test(text)) return `${text}.0`;
  return text;
}

function formatValue(value, kind, language) {
  let text;
  if (kind.kind === "number" || kind.kind === "integer") text = formatNumber(value, language, kind);
  else if (kind.kind === "boolean") {
    if (language === "python") text = value ? "True" : "False";
    else if (language === "r") text = value ? "TRUE" : "FALSE";
    else text = String(value);
  } else if (kind.kind === "text") {
    text = JSON.stringify(value);
    if (language === "rust") text = `${text}.to_string()`;
  } else if (kind.kind === "enum") {
    if (language === "r") text = JSON.stringify(value);
    else if (language === "rust") text = `${kind.entry.name}::${value}`;
    else text = `${kind.entry.name}.${value}`;
  }
  if (language === "rust" && kind.isOptional) return `Some(${text})`;
  return text;
}

export function toSnakeCase(name) {
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

function buildTree(optionValues) {
  const tree = new Map();
  for (const item of optionValues) {
    let level = tree;
    for (const field of item.path.slice(0, -1)) {
      if (!level.has(field)) level.set(field, { children: new Map() });
      level = level.get(field).children;
    }
    level.set(item.path[item.path.length - 1], { item });
  }
  return tree;
}

function formatOptions(tree, optionsType, language, document, imports) {
  const parts = [];
  for (const [field, node] of tree) {
    const name = field.languages[language].name;
    const text = node.item
      ? formatValue(node.item.value, node.item.kind, language)
      : formatOptions(node.children, getKind(field.languages.rust?.type ?? "", document).entry, language, document, imports);
    if (node.item?.kind.kind === "enum") imports.add(node.item.kind.entry.name);
    if (language === "python") parts.push(`${JSON.stringify(name)}: ${text}`);
    else if (language === "r") parts.push(`${name} = ${text}`);
    else parts.push(`${name}: ${text}`);
  }
  if (language === "python") return `{${parts.join(", ")}}`;
  if (language === "js") return `{ ${parts.join(", ")} }`;
  if (language === "r") return `${toSnakeCase(optionsType.name)}(${parts.join(", ")})`;
  imports.add(optionsType.name);
  return `${optionsType.name} { ${[...parts, "..Default::default()"].join(", ")} }`;
}

export function findCall(code, name) {
  const match = new RegExp(`\\b${name}\\(`).exec(code);
  if (!match) return null;
  const start = match.index + match[0].length;
  let depth = 0;
  let quote = null;
  const pieces = [];
  let pieceStart = start;
  for (let position = start; position < code.length; position += 1) {
    const character = code[position];
    if (quote) {
      if (character === "\\") position += 1;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === '"' || character === "'") quote = character;
    else if ("([{".includes(character)) depth += 1;
    else if (")]}".includes(character)) {
      if (depth === 0) {
        const last = code.slice(pieceStart, position);
        if (last.trim()) pieces.push({ start: pieceStart, end: position });
        return { start, end: position, pieces };
      }
      depth -= 1;
    } else if (character === "," && depth === 0) {
      pieces.push({ start: pieceStart, end: position });
      pieceStart = position + 1;
    }
  }
  return null;
}

function getCallArguments(entry, language) {
  const names = [];
  for (const argument of entry.arguments ?? []) {
    const name = (argument.languages[language]?.name ?? "");
    const group = name.includes(".") ? name.split(".")[0] : name;
    if (language === "rust" && group === "self") continue;
    if (names[names.length - 1]?.group === group) continue;
    names.push({ group, argument });
  }
  return names;
}

function splitPiece(code, piece) {
  const text = code.slice(piece.start, piece.end);
  const match = /^(\s*)(?:([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)\s*)?([\s\S]*?)(\s*)$/.exec(text);
  return { before: match[1], name: match[2] ?? null, value: match[3], after: match[4] };
}

function findPiece(code, call, entry, argument, language) {
  const name = (argument.languages[language]?.name ?? "");
  const parsed = call.pieces.map((piece) => ({ piece, ...splitPiece(code, piece) }));
  const named = parsed.find((item) => item.name === name);
  if (named) return named;
  const order = getCallArguments(entry, language);
  const position = order.findIndex((item) => item.argument === argument);
  const candidate = parsed[position];
  if (candidate && !candidate.name) return candidate;
  return null;
}

function findKeyValue(code, piece, key) {
  const text = code.slice(piece.start, piece.end);
  const match = new RegExp(`\\b${key}\\s*:\\s*`).exec(text);
  if (!match) return null;
  const start = piece.start + match.index + match[0].length;
  let depth = 0;
  for (let position = start; position < piece.end; position += 1) {
    const character = code[position];
    if ("([{".includes(character)) depth += 1;
    else if (")]}".includes(character)) {
      if (depth === 0) return { start, end: position };
      depth -= 1;
    } else if (character === "," && depth === 0) return { start, end: position };
  }
  return { start, end: piece.end };
}

export function findDefinition(code, name) {
  const match = new RegExp(`^[ \\t]*(?:const |let |let mut )?${name}\\s*(?:=|<-)\\s*`, "m").exec(code);
  if (!match) return null;
  const start = match.index + match[0].length;
  let depth = 0;
  for (let position = start; position < code.length; position += 1) {
    const character = code[position];
    if ("([{".includes(character)) depth += 1;
    else if (")]}".includes(character)) {
      depth -= 1;
      if (depth === 0) return { start, end: position + 1 };
    } else if (character === "\n" && depth === 0) return { start, end: position };
  }
  return null;
}

function addImports(code, names, language) {
  if (names.size === 0) return code;
  const patterns = {
    python: /^from quantion import (.+)$/m,
    js: /^import \{([^}]*)\} from "quantion";$/m,
    rust: /^use quantion::\{([^}]*)\};$/m,
  };
  const pattern = patterns[language];
  if (!pattern) return code;
  const match = pattern.exec(code);
  if (!match) return code;
  const present = new Set(match[1].split(",").map((item) => item.trim()).filter(Boolean));
  for (const name of names) present.add(name);
  const sorted = [...present].sort();
  if (language === "python") return code.replace(pattern, `from quantion import ${sorted.join(", ")}`);
  if (language === "js") return code.replace(pattern, `import { ${sorted.join(", ")} } from "quantion";`);
  return code.replace(pattern, `use quantion::{${sorted.join(", ")}};`);
}

function namePrefix(name, language) {
  if (!name) return "";
  return language === "r" ? `${name} = ` : `${name}=`;
}

export function buildTemplateSnippet(values, templateValues, document, language) {
  let snippet = buildSnippet(values, document, language);
  if (!snippet.valid) return snippet;
  for (const [name, helperValues] of Object.entries(templateValues ?? {})) {
    const next = buildSnippet({ function: name, ...helperValues }, document, language, snippet.code);
    if (!next.valid) return next;
    snippet = { ...snippet, code: next.code, placed: { ...snippet.placed, ...next.placed } };
  }
  return snippet;
}

export function buildSnippet(values, document, language, baseCode) {
  const checked = validateValues(values, document);
  if (!checked.valid) return { valid: false, reason: checked.reason };
  const { entry, optionsType, argumentValues, optionValues } = checked;
  const example = baseCode ?? entry.languages[language]?.example;
  if (!example) return { valid: false, reason: "no example" };
  const call = findCall(example, getDisplayName(entry, language));
  if (!call) return { valid: false, reason: "no call in example" };

  const edits = [];
  for (const item of argumentValues) {
    const found = findPiece(example, call, entry, item.argument, language);
    const isNumeric = item.kind.kind === "number" || item.kind.kind === "integer";
    if (!found || (!isNumeric && !/^-?\d+(\.\d+)?$/.test(found.value)) || (language === "rust" && item.kind.isPosition)) return { valid: false, reason: `cannot place ${item.argument.languages.python?.name}` };
    const text = formatValue(item.value, item.kind, language);
    const key = (item.argument.languages[language]?.name ?? "").split(".")[1];
    if (key) {
      const inner = findKeyValue(example, found.piece, key);
      if (!inner) return { valid: false, reason: `cannot place ${item.argument.languages.python?.name}` };
      edits.push({ start: inner.start, end: inner.end, text });
      continue;
    }
    edits.push({ start: found.piece.start, end: found.piece.end, text: `${found.before}${namePrefix(found.name, language)}${text}${found.after}` });
  }

  const imports = new Set();
  if (optionValues.length > 0) {
    const literal = formatOptions(buildTree(optionValues), optionsType, language, document, imports);
    const optionsArgument = entry.arguments.find((item) => findByName(document, item.languages.rust?.type ?? "") === optionsType);
    const found = findPiece(example, call, entry, optionsArgument, language);
    const useName = language === "r" || language === "python";
    const definition = found && /^[A-Za-z_][A-Za-z0-9_]*$/.test(found.value) ? findDefinition(example, found.value) : null;
    if (definition) edits.push({ start: definition.start, end: definition.end, text: literal });
    else if (found) edits.push({ start: found.piece.start, end: found.piece.end, text: `${found.before}${namePrefix(found.name, language)}${literal}${found.after}` });
    else if (useName || call.pieces.length === getCallArguments(entry, language).length - 1) edits.push({ start: call.end, end: call.end, text: `, ${namePrefix(useName ? "options" : null, language)}${literal}` });
    else return { valid: false, reason: "cannot place options" };
  }

  edits.sort((left, right) => right.start - left.start);
  let code = example;
  for (const edit of edits) code = code.slice(0, edit.start) + edit.text + code.slice(edit.end);
  const placed = {};
  for (const item of argumentValues) placed[item.argument.languages.python?.name] = item.value;
  for (const item of optionValues) placed[item.path[item.path.length - 1].languages.python.name] = item.value;
  return { valid: true, code: addImports(code, imports, language), function: entry.name, placed };
}

function isValueKind(kind) {
  return ["number", "integer", "boolean", "text", "enum"].includes(kind.kind);
}

export function listOptionFunctions(name, document) {
  return document.entries.filter((entry) => entry.kind === "function" && listValueKeys(entry, document).some((key) => key.isOption && key.item.languages.python.name === name));
}

export function listValueKeys(entry, document) {
  const keys = [];
  for (const argument of entry.arguments ?? []) {
    const kind = getKind(argument.languages.rust?.type ?? "", document);
    if (isValueKind(kind) && argument.languages.python) keys.push({ item: argument, kind, isOption: false });
  }
  const optionsType = getOptionsType(entry, document);
  for (const field of optionsType?.fields ?? []) {
    const kind = getKind(field.languages.rust?.type ?? "", document);
    if (isValueKind(kind)) keys.push({ item: field, kind, isOption: true });
    if (kind.kind !== "options") continue;
    for (const inner of kind.entry.fields ?? []) {
      const innerKind = getKind(inner.languages.rust?.type ?? "", document);
      if (isValueKind(innerKind)) keys.push({ item: inner, kind: innerKind, isOption: true });
    }
  }
  return keys;
}
