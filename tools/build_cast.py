"""Writes tests/features/casts/build.cast, the terminal of the demo: glitu building the demo.

The outputs are those of `glitu build` and `glitu export` run in tests/features; run again after
changing them:

    python3 tools/build_cast.py
"""
from pathlib import Path

from asciicast import BLUE, BOLD, GREEN, RESET, Recording

DEST = Path(__file__).resolve().parent.parent / "tests/features/casts/build.cast"
PROMPT = f"{GREEN}{BOLD}me@desk{RESET}:{BLUE}{BOLD}~/glitu/tests/features{RESET}$ "


def main():
    rec = Recording(cols=72, rows=10, prompt=PROMPT)
    rec.command("glitu build -o dist", ["dist: 35 files"])
    rec.pause("build")
    folders = "  ".join(f"{BLUE}{BOLD}{d}{RESET}" for d in ["fonts", "glitu", "img"])
    rec.type("ls dist")
    rec.line(f"{folders}  index.html  {BLUE}{BOLD}theme{RESET}", 0.3, plain="fonts  glitu  img  index.html  theme")
    rec.prompt()
    rec.pause("ls")
    rec.command("glitu build --standalone -o demo.html", ["demo.html: standalone page, 2032 KB"])
    rec.pause("standalone")
    rec.command("glitu export -o demo.pdf", ["demo.pdf: 37 pages"], first_dt=2.5)
    rec.save(DEST, title="glitu build")


if __name__ == "__main__":
    main()
