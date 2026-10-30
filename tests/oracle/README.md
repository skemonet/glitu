# Rendering oracle

One screenshot per state (slide × click) of `tests/features/slides.md`, the demo of every syntax.
It is the reference of the engine.

`glitu oracle` is a development command: build it with `cargo build --release --features oracle`.

```sh
target/release/glitu serve tests/features/slides.md -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
```

Screenshots are reproducible: `prefers-reduced-motion` turns animations off, `Math.random` is
replayed identically (crystal backdrop), and each state is captured once the screen stops moving
(a terminal typing). A state fails beyond 0.5 % of differing pixels; the image of the differences
is then written next to the rendering (`diff_*.png`).

Text antialiasing depends on the Chromium build and the system fonts: capture and check with the
same browser (`CHROMIUM=/path/to/chrome`).

Diagrams are redrawn at every build by the skemo engine (git dependency of `glitu-core`, pinned to
a revision): bumping skemo can change the slides with diagrams. Re-capture the reference after
looking at the new rendering (the capture keeps this README):
`target/release/glitu oracle capture http://localhost:8123/ tests/oracle`.
