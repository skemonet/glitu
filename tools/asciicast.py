"""Writes asciicast v2 recordings, played step by step by glitu.

A "m" marker ends each segment: the player stops there until the next click.

    from asciicast import Recording
    rec = Recording(cols=78, rows=18, prompt="me@desk:~$ ")
    rec.command("ls", ["a  b"])     # typed, its output, then the next prompt
    rec.pause("ls")                  # the player waits for the click
    rec.save("casts/demo.cast", title="demo")
"""
import json
import random
from pathlib import Path

GREEN, BLUE, CYAN, YELLOW, DIM, BOLD, RESET = "\x1b[32m", "\x1b[34m", "\x1b[36m", "\x1b[33m", "\x1b[2m", "\x1b[1m", "\x1b[0m"


class Recording:
    def __init__(self, cols, rows, prompt, seed=12):
        self.cols, self.rows, self.prompt_text = cols, rows, prompt
        self.events, self.t = [], 0.0
        self.random = random.Random(seed)

    def out(self, data, dt=0.0):
        self.t += dt
        self.events.append([round(self.t, 3), "o", data])

    def line(self, text="", dt=0.04, plain=None):
        """One line of output; `plain`: its text without colours, to check the width"""
        assert len(plain if plain is not None else text) <= self.cols, text
        self.out(text + "\r\n", dt)

    def prompt(self, dt=0.3):
        self.out(self.prompt_text, dt)

    def type(self, cmd):
        """The command typed on the keyboard, key by key, then Enter"""
        for ch in cmd:
            self.out(ch, self.random.uniform(0.045, 0.12))
        self.out("\r\n", 0.4)

    def command(self, cmd, output=(), first_dt=0.5, dt=0.04):
        """A typed command, its output lines, then the next prompt (the first one at the start)"""
        if not self.events:
            self.prompt(0.0)
        self.type(cmd)
        for k, text in enumerate(output):
            self.line(text, first_dt if k == 0 else dt)
        self.prompt()

    def pause(self, label, wait=0.6):
        """End of a segment: the player waits for the next click"""
        self.t += wait
        self.events.append([round(self.t, 3), "m", label])

    def save(self, dest, title, timestamp=None):
        dest = Path(dest)
        dest.parent.mkdir(parents=True, exist_ok=True)
        header = {"version": 2, "width": self.cols, "height": self.rows}
        if timestamp is not None:
            header["timestamp"] = timestamp
        header |= {"title": title, "env": {"TERM": "xterm-256color", "SHELL": "/bin/bash"}}
        with dest.open("w", encoding="utf-8") as f:
            f.write(json.dumps(header, ensure_ascii=False) + "\n")
            for e in self.events:
                f.write(json.dumps(e, ensure_ascii=False) + "\n")
        markers = sum(1 for e in self.events if e[1] == "m")
        print(f"{dest}: {len(self.events)} events, {self.t:.1f} s, {markers} markers → {markers + 1} .cast-step")
