---
title: Presenting
description: Keyboard shortcuts, the speaker view with notes and timer, rehearsals, synchronized windows, a phone as a remote control, drawing on slides and the scroll view.
order: 6
---

## Keyboard shortcuts

<kbd>?</kbd> shows this list during the presentation.

| Key | Action |
|---|---|
| <kbd>→</kbd> <kbd>Space</kbd> <kbd>N</kbd> | Next step or slide, in reading order |
| <kbd>←</kbd> <kbd>Shift</kbd>+<kbd>Space</kbd> <kbd>P</kbd> | Previous step or slide |
| <kbd>Page↓</kbd> <kbd>Page↑</kbd> | Next or previous chapter in 2D navigation, otherwise next or previous slide |
| <kbd>↓</kbd> <kbd>↑</kbd> | Within the chapter; <kbd>H</kbd> <kbd>J</kbd> <kbd>K</kbd> <kbd>L</kbd> for left, down, up, right |
| <kbd>Home</kbd> <kbd>End</kbd> | First or last slide |
| <kbd>G</kbd> | Go to a slide: its number, `5.2`, or its identifier |
| <kbd>Esc</kbd> <kbd>O</kbd> | Overview: choose with the arrows, then <kbd>Enter</kbd> or a click |
| <kbd>F</kbd> | Full screen |
| <kbd>S</kbd> | Speaker view |
| <kbd>D</kbd> | Draw on the slide |
| <kbd>B</kbd> <kbd>V</kbd> <kbd>.</kbd> | Black screen, then resume |
| <kbd>A</kbd> | Pause or resume the automatic advance |
| <kbd>Alt</kbd>+click (<kbd>Ctrl</kbd>+click on Linux) | Zoom on an element, then back |

On a touch screen, swipe with one finger to change slides, and use two fingers to zoom and pan.
The mouse wheel changes slides when the theme's `glitu.yaml` sets `mouseWheel: true`.

## Speaker view

<kbd>S</kbd> opens the speaker view in a separate window. It shows the current slide as the
audience sees it, the next step, the notes, the clock and a timer. The passage of the notes that
matches the current click is highlighted (see `[click]` in [Writing slides](/docs/writing/#speaker-notes)).

All the shortcuts work in the speaker view, so you can drive the presentation from it. Drag the
separators to resize the columns and use <kbd>A−</kbd> <kbd>A+</kbd> to change the size of the
notes; the browser remembers the layout. The timer survives a reload or a closed speaker window,
and `⋯` resets it or changes the layout.

## Timing and rehearsals

A planned duration is written at the end of a slide title:

```markdown
## Live demo ⏳95🏃3
```

The slide is planned for 95 seconds, averaged over 3 rehearsals (`🏃3` is optional). Under the
current slide, the speaker view lists the planned and actual time of every slide, and its bottom
bar shows whether you are ahead of or behind schedule.

To time a rehearsal, run `glitu server`, open the speaker view, then choose `⋯` → **Rehearse from
the start** (or **from here**). <kbd>↺</kbd> restarts the current slide and <kbd>⏸</kbd> pauses.
**Finish** shows the time measured for each slide and what to do with it: average it with the
previous rehearsals (the default), replace the planned duration, or skip it. glitu then writes the
durations into the slide titles of the Markdown file, so save your edits first.

## Synchronized windows and remote control

Under `glitu server`, every window that shows the presentation stays on the same slide: projector,
laptop and tablet follow each other.

To use a phone as a remote control, allow your local network when starting the server:

```sh
glitu server --allow 192.168.0.0/16
```

Then open the address shown, followed by `?remote`, on the phone. The remote shows the current
slide, two large buttons and the notes, and joins the presentation where it is. `?follow` shows
the presentation in sync, without the buttons.

## Drawing

<kbd>D</kbd> turns the slide into a drawing surface for the mouse, a finger or a stylus. Keys
<kbd>1</kbd> to <kbd>5</kbd> choose the colour, <kbd>Z</kbd> undoes, <kbd>X</kbd> clears, and
<kbd>D</kbd> or <kbd>Esc</kbd> leaves. The drawings stay with their slide and the browser keeps them
from one session to the next.

## Scroll view

Adding `?view=scroll` to the address shows the slides one below the other, at the width of the
window, with every step visible: the presentation reads like a web page. Phones (windows narrower
than 435 px) get this view by default, except as a remote; in landscape, they switch back to the
presentation.

## Printing

Adding `?print-pdf` to the address lays out one slide per page with every step visible; print it
to PDF from the browser. `glitu build pdf -o talk.pdf` does the same from the command line: see
[Building and exporting](/docs/publishing/#pdf).
