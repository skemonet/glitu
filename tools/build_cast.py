"""Writes the terminals of the decks: demo/casts/build.cast, glitu building the demo, and
tests/features/casts/build.cast, glitu building the features deck.

The outputs are those of `glitu build site`, `glitu build`, `glitu build pdf` and `glitu build pptx` run in
demo/ and tests/features; run again after changing them:

    python3 tools/build_cast.py
"""
from pathlib import Path

from asciicast import BLUE, BOLD, GREEN, RESET, Recording

ROOT = Path(__file__).resolve().parent.parent
PROMPT = f"{GREEN}{BOLD}me@desk{RESET}:{BLUE}{BOLD}~/glitu/tests/features{RESET}$ "
DEMO_PROMPT = f"{GREEN}{BOLD}me@desk{RESET}:{BLUE}{BOLD}~/talks/demo{RESET}$ "


def demo():
    rec = Recording(cols=72, rows=8, prompt=DEMO_PROMPT)
    rec.command("glitu build -o demo.html", ["Built demo.html (2.8 MB)"])
    rec.pause("page")
    rec.command("glitu build pdf -o demo.pdf", ["Built demo.pdf (12 pages)"], first_dt=2.5)
    rec.pause("pdf")
    rec.command("glitu build pptx -o for-the-boss.pptx", ["Built for-the-boss.pptx (12 slides)"], first_dt=2.5)
    rec.save(ROOT / "demo/casts/build.cast", title="glitu build")


def features():
    rec = Recording(cols=72, rows=10, prompt=PROMPT)
    rec.command("glitu build site -o dist", ["Built dist (static website, 38 files)"])
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
    rec.command("glitu build -o demo.html", ["Built demo.html (2.5 MB)"])
    rec.pause("standalone")
    rec.command("glitu build pdf -o demo.pdf", ["Built demo.pdf (48 pages)"], first_dt=2.5)
    rec.save(ROOT / "tests/features/casts/build.cast", title="glitu build")


if __name__ == "__main__":
    demo()
    features()
