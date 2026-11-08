"""Writes tests/features/casts/build.cast, the terminal of the demo: glitu building the demo.

The outputs are those of `glitu build site`, `glitu build` and `glitu build pdf` run in tests/features; run again after
changing them:

    python3 tools/build_cast.py
"""
from pathlib import Path

from asciicast import BLUE, BOLD, GREEN, RESET, Recording

DEST = Path(__file__).resolve().parent.parent / "tests/features/casts/build.cast"
PROMPT = f"{GREEN}{BOLD}me@desk{RESET}:{BLUE}{BOLD}~/glitu/tests/features{RESET}$ "


def main():
    rec = Recording(cols=72, rows=10, prompt=PROMPT)
    rec.command("glitu build site -o dist", ["Built dist (static website, 36 files)"])
    rec.pause("build")
    folders = "  ".join(f"{BLUE}{BOLD}{d}{RESET}" for d in ["fonts", "glitu", "img"])
    rec.type("ls dist")
    rec.line(
        f"{folders}  index.html  page.html  {BLUE}{BOLD}theme{RESET}",
        0.3,
        plain="fonts  glitu  img  index.html  page.html  theme",
    )
    rec.prompt()
    rec.pause("ls")
    rec.command("glitu build -o demo.html", ["Built demo.html (2.0 MB)"])
    rec.pause("standalone")
    rec.command("glitu build pdf -o demo.pdf", ["Built demo.pdf (37 pages)"], first_dt=2.5)
    rec.save(DEST, title="glitu build")


if __name__ == "__main__":
    main()
