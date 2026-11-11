import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const jsRoot = path.join(root, "wrappers", "js");
const require = createRequire(path.join(jsRoot, "package.json"));
const ts = require("typescript");

const entries = ["src/index-node.ts", "src/index-wasm.ts"].map((entry) => path.join(jsRoot, entry));
const program = ts.createProgram(entries, {
  target: ts.ScriptTarget.ES2020,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Node10,
  strict: true,
  skipLibCheck: true,
  noEmit: true,
});
const checker = program.getTypeChecker();
const flags = ts.TypeFormatFlags.NoTruncation | ts.TypeFormatFlags.UseFullyQualifiedType;

function clean(text) {
  return text.replace(/import\("[^"]*"\)\./g, "").replace(/\s+/g, " ");
}

function describeMembers(type) {
  return checker
    .getPropertiesOfType(type)
    .filter((member) => !member.getName().startsWith("_") && !member.getName().startsWith("#"))
    .filter((member) => {
      const declaration = member.valueDeclaration ?? member.declarations?.[0];
      const modifiers = declaration ? ts.getCombinedModifierFlags(declaration) : 0;
      return !(modifiers & (ts.ModifierFlags.Private | ts.ModifierFlags.Protected));
    })
    .map((member) => {
      const declaration = member.valueDeclaration ?? member.declarations?.[0];
      const optional = member.flags & ts.SymbolFlags.Optional ? "?" : "";
      const memberType = checker.getTypeOfSymbolAtLocation(member, declaration);
      return `    ${member.getName()}${optional}: ${clean(checker.typeToString(memberType, declaration, flags))}`;
    })
    .sort();
}

function describe(symbol) {
  const target = symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol;
  const name = symbol.getName();
  const declaration = target.valueDeclaration ?? target.declarations?.[0];
  const lines = [];
  if (target.flags & ts.SymbolFlags.Function) {
    const type = checker.getTypeOfSymbolAtLocation(target, declaration);
    for (const signature of type.getCallSignatures()) {
      lines.push(`function ${name}${clean(checker.signatureToString(signature, declaration, flags))}`);
    }
  } else if (target.flags & ts.SymbolFlags.Class) {
    lines.push(`class ${name}`);
    lines.push(...describeMembers(checker.getDeclaredTypeOfSymbol(target)));
  } else if (target.flags & ts.SymbolFlags.Interface) {
    lines.push(`interface ${name}`);
    lines.push(...describeMembers(checker.getDeclaredTypeOfSymbol(target)));
  } else if (target.flags & ts.SymbolFlags.TypeAlias) {
    lines.push(`type ${name} = ${clean(checker.typeToString(checker.getDeclaredTypeOfSymbol(target), declaration, flags))}`);
  }
  if (target.flags & ts.SymbolFlags.Variable) {
    const type = checker.getTypeOfSymbolAtLocation(target, declaration);
    lines.push(`const ${name}: ${clean(checker.typeToString(type, declaration, flags))}`);
  }
  return lines;
}

const output = [];
for (const entry of entries) {
  const source = program.getSourceFile(entry);
  const module = checker.getSymbolAtLocation(source);
  output.push(`entry ${path.relative(jsRoot, entry).split(path.sep).join("/")}`);
  output.push("");
  const exports = checker.getExportsOfModule(module).sort((left, right) => left.getName().localeCompare(right.getName()));
  for (const symbol of exports) {
    output.push(...describe(symbol), "");
  }
}
process.stdout.write(output.join("\n"));
