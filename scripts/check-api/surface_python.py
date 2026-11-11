import enum
import inspect
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "wrappers" / "python"))

import quantion


def describe_type(name, value):
    if isinstance(value, type) and issubclass(value, enum.Enum):
        lines = [f"enum {name}"]
        lines.extend(f"    {member.name} = {member.value!r}" for member in value)
        return lines
    if isinstance(value, type) and hasattr(value, "__annotations__") and hasattr(value, "__total__"):
        lines = [f"typeddict {name} total={value.__total__}"]
        required = getattr(value, "__required_keys__", set())
        for key, annotation in sorted(value.__annotations__.items()):
            marker = "" if key in required else "?"
            lines.append(f"    {key}{marker}: {format_annotation(annotation)}")
        return lines
    if isinstance(value, type):
        lines = [f"class {name}({', '.join(base.__name__ for base in value.__bases__)})"]
        for member_name, member in sorted(vars(value).items()):
            if member_name.startswith("_") and member_name not in ("__init__", "__enter__", "__exit__"):
                continue
            if callable(member):
                lines.append(f"    def {member_name}{signature(member)}")
            elif isinstance(member, property):
                lines.append(f"    property {member_name}")
        return lines
    if callable(value):
        return [f"def {name}{signature(value)}"]
    return [f"value {name} = {value!r}"]


def format_annotation(annotation):
    if isinstance(annotation, str):
        return annotation
    if hasattr(annotation, "__forward_arg__"):
        return annotation.__forward_arg__
    if isinstance(annotation, type):
        return annotation.__name__
    return str(annotation).replace("typing.", "").replace("quantion._types.", "")


def signature(function):
    try:
        return str(inspect.signature(function)).replace("typing.", "")
    except (TypeError, ValueError):
        return "(...)"


def normalize(line):
    line = re.sub(r"ForwardRef\('([^']*)'[^)]*\)", r"\1", line)
    line = line.replace("'", "").replace('"', "")
    previous = None
    while previous != line:
        previous = line
        line = re.sub(r"Union\[([^\[\]]*)\]", lambda match: " | ".join(part.strip() for part in match.group(1).split(",")), line)
        line = re.sub(r"Optional\[([^\[\]]*)\]", r"\1 | None", line)
    return line


def main():
    lines = []
    for name in sorted(quantion.__all__):
        lines.extend(describe_type(name, getattr(quantion, name)))
        lines.append("")
    sys.stdout.write("\n".join(normalize(line) for line in lines))


if __name__ == "__main__":
    main()
