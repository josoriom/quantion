import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "core" / "src"


def read_exports():
    text = (SOURCE / "lib.rs").read_text(encoding="utf-8")
    exports = []
    for statement in re.findall(r"pub use ([^;]+);", text, re.S):
        statement = re.sub(r"\s+", "", statement)
        exports.extend(expand(statement))
    return sorted(set(exports))


def expand(statement):
    if "{" not in statement:
        path, _, name = statement.rpartition("::")
        return [(path, name)]
    head, _, rest = statement.partition("{")
    head = head.rstrip(":")
    body = rest[:-1]
    items, depth, current = [], 0, ""
    for character in body:
        if character == "{":
            depth += 1
        if character == "}":
            depth -= 1
        if character == "," and depth == 0:
            items.append(current)
            current = ""
        else:
            current += character
    if current:
        items.append(current)
    found = []
    for item in items:
        found.extend(expand(f"{head}::{item}" if head else item))
    return found


def module_file(path):
    parts = path.split("::")
    candidate = SOURCE.joinpath(*parts).with_suffix(".rs")
    if candidate.exists():
        return candidate
    return SOURCE.joinpath(*parts, "mod.rs")


def read_until_body(text, start):
    end = text.index("{", start)
    signature = " ".join(text[start:end].split())
    return signature.replace("( ", "(").replace(", )", ")")


def read_block(text, start):
    open_brace = text.index("{", start)
    depth, index = 0, open_brace
    while True:
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace + 1:index]
        index += 1


def public_lines(block, pattern):
    lines = []
    for line in block.split("\n"):
        line = line.strip()
        if re.match(pattern, line):
            lines.append(" ".join(line.rstrip(",").split()))
    return lines


def read_definition_text(path, name):
    text = module_file(path).read_text(encoding="utf-8").split("\n#[cfg(test)]")[0]
    pattern = r"pub use ([a-z_0-9]+)::(?:\{[^}]*\b" + name + r"\b[^}]*\}|" + name + r")\s*;"
    reexport = re.search(pattern, text)
    if reexport:
        return read_definition_text(f"{path}::{reexport.group(1)}", name)
    return text


def describe(path, name):
    text = read_definition_text(path, name)
    out = []
    function = re.search(r"pub fn " + name + r"\b", text)
    struct = re.search(r"pub struct " + name + r"\b", text)
    enum = re.search(r"pub enum " + name + r"\b", text)
    if function:
        out.append(read_until_body(text, function.start()))
    elif struct:
        header = read_until_body(text, struct.start()) if "{" in text[struct.start():struct.start() + 200].split(";")[0] else text[struct.start():text.index(";", struct.start())]
        out.append(" ".join(header.split()))
        if not header.endswith(";") and "{" in text[struct.start():text.index("\n", struct.start()) + 1]:
            for field in public_lines(read_block(text, struct.start()), r"pub [a-z_0-9]+:"):
                out.append("    " + field)
    elif enum:
        out.append(read_until_body(text, enum.start()))
        for variant in public_lines(read_block(text, enum.start()), r"[A-Z][A-Za-z0-9_]*\b"):
            out.append("    " + variant)
    else:
        out.append(f"missing {path}::{name}")
    return out


def describe_impls(name):
    lines = []
    for file in sorted(SOURCE.rglob("*.rs")):
        if "tests" in file.parts:
            continue
        text = file.read_text(encoding="utf-8").split("\n#[cfg(test)]")[0]
        for match in re.finditer(r"\nimpl(?:<[^>]*>)? (?:([A-Za-z0-9_<>&': ]+) for )?" + name + r"\b[^{]*\{", text):
            trait = match.group(1)
            block = read_block(text, match.start())
            if trait == "Default":
                fields = public_lines(block, r"[a-z_0-9]+:")
                lines.append(f"impl Default for {name}: " + ", ".join(fields))
            elif trait:
                lines.append(f"impl {' '.join(trait.split())} for {name}")
            else:
                for method in re.finditer(r"\n\s*pub fn ", block):
                    lines.append("    " + read_until_body(block, method.start() + 1).replace("pub fn", f"{name}::fn", 1))
    return sorted(lines)


def main():
    lines = []
    for path, name in read_exports():
        lines.extend(describe(path, name))
        lines.extend(describe_impls(name))
        lines.append("")
    sys.stdout.write("\n".join(lines))


if __name__ == "__main__":
    main()
