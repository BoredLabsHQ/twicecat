#!/usr/bin/env python3
"""Design guardrails for CI. Exits 1 on any failure.

    python3 design/scripts/check_design.py [path ...]   # paths to scan, default: crates/ src/

1. Contrast: every text pair the brand book promises still meets its floor.
2. No colour literals in Rust UI code outside design/generated/: use theme::color::*.

Standard library only.
"""
import json, pathlib, re, sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
TOKENS = ROOT / "design" / "tokens.json"

# (foreground, background, minimum ratio) — keep in sync with design/README.md
PAIRS = [(fg, bg, 4.5) for fg in ("ink", "ink-muted", "ink-faint", "violet-bright", "live", "ok", "warn")
         for bg in ("void", "surface", "surface-raised", "surface-overlay")]
PAIRS += [("on-violet", "violet", 4.5), ("ink", "violet-deep", 4.5)]
PAIRS += [("line-strong", bg, 3.0) for bg in ("surface", "surface-raised")]
PAIRS += [("focus", bg, 3.0) for bg in ("void", "surface", "surface-raised", "surface-overlay")]

LITERAL = re.compile(r'"#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?"|\bColor(?:32)?::from_rgba?(?:8|_u8|_premultiplied|_unmultiplied)?\(\s*\d'
                     r'|\brgba?!?\(\s*\d|\b0x[0-9a-fA-F]{6}\b')


def lum(c):
    def ch(x):
        return x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(x) for x in c["components"])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def main():
    colors = {k: v["$value"] for k, v in json.loads(TOKENS.read_text(encoding="utf-8"))["color"].items() if not k.startswith("$")}
    failed = False
    for fg, bg, floor in PAIRS:
        r = ratio(colors[fg], colors[bg])
        if r < floor:
            failed = True
            print(f"contrast: {fg} on {bg} is {r:.2f}:1, needs {floor}:1")
    roots = [ROOT / p for p in (sys.argv[1:] or ["crates", "src"])]
    for root in roots:
        for f in root.rglob("*.rs") if root.exists() else []:
            if "design/generated" in f.as_posix() or "/target/" in f.as_posix():
                continue
            for n, line in enumerate(f.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
                if LITERAL.search(line) and "design-allow" not in line:
                    failed = True
                    print(f"{f.relative_to(ROOT)}:{n}: colour literal — use theme::color::* (or add `// design-allow` with a reason)")
    print("design check: FAILED" if failed else "design check: ok")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
