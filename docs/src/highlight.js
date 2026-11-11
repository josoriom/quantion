const keywords = {
  rust: "use let mut fn pub impl for in if else return match struct enum as ref where self Self crate",
  python: "import from def return for in if else with as and or not lambda class",
  js: "import from const let var function async await return export new for of if else typeof interface type",
  r: "library function if else for in return",
  shell: "pip cargo npm add install",
};

const literals = {
  rust: "true false Some None Ok Err",
  python: "True False None",
  js: "true false null undefined",
  r: "TRUE FALSE NULL NaN NA Inf",
  shell: "",
};

const patterns = {
  rust: /(\/\/.*)|("(?:\\.|[^"\\])*")|(\b\d+(?:\.\d+)?(?:e-?\d+)?\b)|(&mut\b|->|=>|::|\?|&)|\b([a-z_]\w*)\b(?=\s*[(!<])|\b([A-Z]\w*)\b|\b([a-z_]\w*)\b/g,
  python: /(#.*)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')|(\b\d+(?:\.\d+)?(?:e-?\d+)?\b)|(->|\*)|\b([A-Za-z_]\w*)\b(?=\s*\()|\b([A-Z]\w*)\b|\b([a-z_]\w*)\b/g,
  js: /(\/\/.*)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`[^`]*`)|(\b\d+(?:\.\d+)?(?:e-?\d+)?\b)|(=>|\.\.\.)|\b([A-Za-z_$][\w$]*)\b(?=\s*\()|\b([A-Z][\w$]*)\b|\b([a-z_$][\w$]*)\b/g,
  shell: /(#.*)|("(?:\\.|[^"\\])*")|(\b\d+(?:\.\d+)?\b)|(--?[a-z-]+)|((?!x)x)|((?!x)x)|\b([a-z_][\w-]*)\b/g,
  r: /(#.*)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')|(\b\d+(?:\.\d+)?(?:e-?\d+)?L?\b)|(<-|\$|::)|\b([A-Za-z_.][\w.]*)\b(?=\s*\()|\b([A-Z][\w.]*)\b|\b([a-z_.][\w.]*)\b/g,
};

function getWordKind(word, language, isCall, isType) {
  if (keywords[language].split(" ").includes(word)) return "keyword";
  if (literals[language].split(" ").includes(word)) return "number";
  if (isCall) return "function";
  if (isType) return "type";
  return null;
}

function getTokenKind(match, language) {
  const [, comment, string, number, operator, call, type, word] = match;
  if (comment) return "comment";
  if (string) return "string";
  if (number) return "number";
  if (operator) return "operator";
  return getWordKind(call ?? type ?? word, language, Boolean(call), Boolean(type));
}

export function splitIntoTokens(code, language) {
  if (!patterns[language]) return [{ text: code }];
  const tokens = [];
  let position = 0;
  for (const match of code.matchAll(new RegExp(patterns[language].source, "g"))) {
    if (match.index > position) tokens.push({ kind: null, text: code.slice(position, match.index) });
    const [text] = match;
    tokens.push({ kind: getTokenKind(match, language), text });
    position = match.index + text.length;
  }
  if (position < code.length) tokens.push({ kind: null, text: code.slice(position) });
  return tokens;
}
