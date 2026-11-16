# Video export

Status: implemented (2026-11-16), first version: §4 and §5 as written; §7 remains open.

`glitu build video` records the presentation as a video file, one chapter per click. `glitu-play`
recognises the file and presents it through mpv, which pauses at each chapter. In any other player,
the file is an ordinary video whose chapters are the clicks.

## 1. Why

- **Faithful**: the browser renders every frame once, at export. The transitions, zooms, terminals
  and embedded videos are played back as they were recorded, with nothing computed at the show.
- **Light**: a video decoder, often in hardware, is all the show needs. That is less than a browser,
  and less than `glitu-play`'s software animations.
- **Portable**: one file, which any computer can play, even without glitu.

What it loses: what needs the page live (speaker view, drawing, remote control, links, iframes).
§7 lists what could come back.

## 2. Why a script

No common video format pauses by itself. DVD and Blu-ray menus can wait for the viewer, but they are
out of reach (DVD is 720 × 576 MPEG-2, Blu-ray menus are heavy to produce). Matroska defines commands
in its chapters, but players ignore them.

Every format can carry chapters, though, and mpv runs Lua scripts that can watch them. A script
of about thirty lines pauses on entering each chapter and maps the presenter's keys.

## 3. The file

### Container and codec — [proposed]

- **Matroska (`.mkv`)**: chapters, tags and attachments, all standard, and read by every player
  that matters. MP4 chapters are less consistent across players.
- **H.264, `yuv420p`, 30 frames per second, at the export size (1920 × 1080 by default)**: plays
  everywhere, decoded in hardware almost everywhere. A slide that does not move costs almost nothing:
  the expected size is a few MB per minute. AV1 can come later as an option.

### Chapters — [proposed]

One chapter per click state, named `slide.click` (`3.2`) with the slide's title. Each chapter starts
with a **hold**: the state of the click, frozen for one second. The animation to the next click
follows.

```
 chapter 1.0           chapter 1.1                chapter 2.0
 ├─ hold (1 s) ─┤─ animation of click 1 ─├─ hold (1 s) ─┤─ transition ─…
```

A player always pauses a few milliseconds after the chapter's start. The hold makes that delay
invisible: the pause falls on identical frames.

### Marks for glitu-play — [proposed]

A global Matroska tag `GLITU_PLAY=1` (with `GLITU_VERSION`) tells `glitu-play` and the script that
the chapters are clicks. An ordinary video with chapters is left alone.

### Speaker notes — [proposed]

A subtitle track named `notes`, disabled by default, with each note over its chapter: they are
there for whoever wants them, and they never show on the projector by accident.

## 4. Producing it: `glitu build video`

### Recording — [proposed]

Recording the screen in real time would drop frames depending on the machine. The export instead
drives Chromium frame by frame:

- `chrome-headless-shell` (the browser glitu already downloads) started with
  `--deterministic-mode --enable-begin-frame-control`. Each `HeadlessExperimental.beginFrame`
  moves the page's clock forward by exactly 1/30 s and returns that frame. The result does not depend
  on how fast the machine is.
- The animated backdrop already replays `Math.random`: the same deck gives the same video.
- **Embedded videos** do not follow that clock. Before each frame, the export sets their
  `currentTime` itself, and waits for the frame to be decoded (`requestVideoFrameCallback`).
- **The end of a click**: the engine says when its animations are done (transitions, typing of a
  terminal, the run of a video to its next pause). That is the end of the chapter's animation. It is
  capped (60 s), so that a looping video cannot record forever.

### Encoding — [proposed]

The frames go to **ffmpeg** through a pipe (raw frames on its standard input), which encodes them and
writes the chapters, tags and notes. There is no H.264 encoder in pure Rust, and writing one is out of
scope. ffmpeg is a tool of the machine, looked up like the diagram tools (`--tool ffmpeg=…`,
`GLITU_TOOL_FFMPEG`, `config.yaml`), and only when `build video` is asked for.

Options: `--fps` (30), `--hold` (1 s), the size of the export (`--size`, as for the other formats),
`--range` (as for `png`).

### Sound — [proposed]

None in the first version. The frame-by-frame clock has no audio, and the embedded videos' sound
would have to be cut and put back in place from their files. That can come later.

## 5. Playing it: `glitu-play talk.mkv`

`glitu-play` reads the file's first bytes. For a Matroska file carrying `GLITU_PLAY=1`, it runs mpv
instead of its own windows:

```sh
mpv --no-config --load-scripts=no --script=<glitu.lua> --fs --keep-open --no-osc \
    --hwdec=auto-safe --sub=no talk.mkv
```

- **The script is inside the `glitu-play` binary** (`include_str!`), written to a temporary file
  for mpv. mpv will not run a script stored inside the video, for security, and that is right.
- `--no-config --load-scripts=no`: the user's own mpv setup and scripts cannot change the keys or
  the display of the show.
- The options of `glitu-play` carry over: `--window` drops `--fs`; `--start <SLIDE>` starts at
  chapter `<SLIDE>.0`; `--speaker` is refused, with a message (§7).
- mpv missing: the error explains how to install it, and that any player can still show the file
  by chapters.

### The script — [proposed]

- On entering a chapter: pause.
- Next (`→ ↓ Space Enter PageDown n`, the click of the mouse): resume, up to the next chapter.
- Previous (`← ↑ Backspace PageUp p`): the start of the previous chapter, then pause.
- `Home End`: first and last chapter. `f F11`: full screen. `q`: quit. These are the keys of
  `glitu-play`: a presenter who knows one knows the other.
- `c`: continuous play, without the pauses, for a looping stand.
- Inactive when the file has no `GLITU_PLAY` tag, so that it can be installed for all of mpv.

## 6. Without glitu-play

The documentation (`site/docs/presenting.md`) explains it:

- `glitu-play --mpv-script > glitu.lua` writes the script, then the mpv command line above, or a
  copy into `~/.config/mpv/scripts/` (the tag keeps it from acting on other videos).
- In VLC or any other player: the file plays, and the chapters are the clicks (VLC: *Playback →
  Chapter*). There is no automatic pause.

## 7. Open questions

- **The hold**: 1 s by default? It also sets how long the pause lasts in a player without the script.
- **The backdrop during a pause**: frozen (simple). The alternative is a few seconds of backdrop
  recorded and looped by the script during the pause: livelier, but heavier.
- **Speaker view**: mpv shows one window. Options: a second mpv window on the notes track, or a small
  window of `glitu-play` that follows mpv through its IPC (`--input-ipc-server`).
- **A page player**: a light HTML page that reads the same video with the same pauses (`<video>`
  and the chapters' times), for a computer without mpv. The engine already knows how to pause a video
  (`pauses=`).
- **Sound** of the embedded videos (§4).

## 8. Order of work

1. The format: chapters, tags and hold, written by hand with ffmpeg on a test video; the script, and
   checking it in mpv (exact pause, back, keys).
2. `glitu-play`: recognising the file, running mpv with the embedded script, `--mpv-script`.
3. `glitu build video`: frame-by-frame recording, then the pipe to ffmpeg. The demo
   (`demo/slides.md`) is the test deck: zoom, terminal, embedded video with pauses.
4. Speaker notes as subtitles; the documentation (§6).
5. Later: the page player, the sound, the speaker view.
