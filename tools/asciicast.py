"""Écriture d'enregistrements asciicast v2 rejoués pas à pas par glitu.

Un marker "m" clôt chaque segment : le lecteur s'y arrête jusqu'au clic suivant.

    from asciicast import Recording
    rec = Recording(cols=78, rows=18, prompt="moi@poste:~$ ")
    rec.command("ls", ["a  b"])     # tapée, sa sortie, puis l'invite suivante
    rec.pause("ls")                  # le lecteur attend le clic
    rec.save("casts/demo.cast", title="démo")
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
        """Une ligne de sortie ; `plain` : son texte sans couleurs, pour vérifier la largeur"""
        assert len(plain if plain is not None else text) <= self.cols, text
        self.out(text + "\r\n", dt)

    def prompt(self, dt=0.3):
        self.out(self.prompt_text, dt)

    def type(self, cmd):
        """La commande tapée au clavier, touche par touche, puis Entrée"""
        for ch in cmd:
            self.out(ch, self.random.uniform(0.045, 0.12))
        self.out("\r\n", 0.4)

    def command(self, cmd, output=(), first_dt=0.5, dt=0.04):
        """Commande tapée, ses lignes de sortie, puis l'invite suivante (la première au début)"""
        if not self.events:
            self.prompt(0.0)
        self.type(cmd)
        for k, text in enumerate(output):
            self.line(text, first_dt if k == 0 else dt)
        self.prompt()

    def pause(self, label, wait=0.6):
        """Fin de segment : le lecteur attend le clic suivant"""
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
        print(f"{dest}: {len(self.events)} événements, {self.t:.1f} s, {markers} markers → {markers + 1} .cast-step")
