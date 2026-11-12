---
title: Slow computers
description: Present on an old or slow computer without a browser, with a PDF in pdfpc or with glitu-play, the native player that keeps the arrows drawn step by step and the terminals typing.
order: 9
---

A glitu presentation runs in a browser, and on an old netbook, the browser is what becomes too
slow: the page lays out, blurs, shades and animates every slide. On such a computer, present
without it. glitu prepares the presentation **on another computer** (the export needs Chromium),
and the slow one only shows pictures:

| | PDF + pdfpc | glitu-play |
|---|---|---|
| What is installed on the slow computer | a PDF reader: pdfpc, mupdf, zathura… | `glitu-play` (4 MB, no browser) |
| Steps | one page per click | one picture per click |
| skemo arrows drawn step by step | no, they appear | yes |
| Terminal recordings typing | no, the result appears | yes |
| Speaker notes, timer | yes, in pdfpc | yes, `s` or `--speaker` |

The pictures are those of the browser, pixel for pixel: the slide looks the same as in the
browser.

## Export at the screen's size

Find the size of the slow computer's screen (`xrandr` on Linux), and export the pictures at its
width with `--width`. The pictures are then shown as they are, without being resized while you
present, and the files are lighter. For a 1024 × 600 screen and 16:9 slides:

```sh
glitu build pdf slides.md --clicks --width 1024 -o talk.pdf    # PDF
glitu build play slides.md --width 1024 -o talk.glituplay     # glitu-play
```

With slides of another shape (for example 4:3 on a 16:9 screen), choose the width that fits the
height: a 960 × 700 slide on a 1024 × 600 screen is 823 pixels wide.

## A PDF, presented with pdfpc

`--clicks` makes one page per click, each a picture: a PDF reader only has to show it, which even
a slow processor does at once. The pages of one slide carry the slide's number, so the readers
that use page labels count slides, not clicks.

[pdfpc](https://pdfpc.github.io/) (`apt install pdfpc`) adds the speaker view on a second screen:
the current and the next slide, the notes, a timer. glitu writes the notes into `talk.pdfpc`,
beside `talk.pdf`; keep the two files together.

```sh
pdfpc talk.pdf          # the presentation on the projector, the speaker view on the laptop
pdfpc -s talk.pdf       # the other way round
mupdf talk.pdf          # or any reader, full screen (mupdf: f)
```

Without `--clicks`, the PDF is printed as vectors: its text can be selected, but a slow computer
has to draw each page, and the steps are lost. For presenting, prefer `--clicks`.

## glitu-play, the native player

`glitu-play` is a small program, separate from `glitu`, that presents the `.glituplay` file written by
`glitu build play` without a browser. It shows the pictures of the clicks, and animates by itself
only what moves:

- the **skemo arrows** drawn step by step: the browser's own pixels revealed along the arrow, then
  its head, as in the browser;
- the **terminal recordings**: the typing recorded during the export, image by image, at its
  pace;
- the other changes of a click fade in; the next slide fades over the previous one.

The last picture of each animation is exactly the browser's. Only the area that changes is
redrawn, by software: no graphics card is needed.

```sh
glitu-play talk                  # full screen
glitu-play talk --window         # in a window
glitu-play talk --start 12       # from slide 12
glitu-play talk --speaker        # with the speaker view
```

| Key | Action |
|---|---|
| → ↓ Space Enter Page Down `n`, click | Next |
| ← ↑ Backspace Page Up `p` | Previous |
| Home, End | First, last click |
| `f`, F11 | Full screen |
| Esc | Leave full screen |
| `s` | Speaker view: open, close |
| `t`, `r` | Timer: pause or resume, back to zero |
| `+`, `-` | Size of the notes |
| `q` | Quit |

A click during an animation ends it and goes to the next one. Going back shows the previous click
at once.

The speaker view opens in a second window: the current click and the next one, the notes, the
slide and its click, and a timer that starts at the first click forward. With two screens it goes
full screen on the other one; with `--speaker`, the slides go to the secondary screen and the view
to the primary one (the laptop's). The keys work in both windows.

The directory holds one PNG per click (and a few more for the arrows and the terminals), and a
`deck.json` that describes them; copy the whole directory to the slow computer.

Build `glitu-play` from the sources of glitu with `cargo build --release -p glitu-play`; it runs
on Linux (X11 or Wayland), macOS and Windows.

## What is not there yet

- In glitu-play: the animated backdrop (it is still), animated code and
  animated slides (they fade), drawing on the slides.
- Rendering with the graphics card in glitu-play, which can be turned off.
