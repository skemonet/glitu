# Oracle de rendu

Une capture par état (slide × clic) de `tests/fonctions/slides.md`, la démonstration de toutes
les syntaxes. C'est la référence du moteur ; capturée le 2026-10-26 (elle portait avant sur une
ancienne présentation Kubernetes, retirée du dépôt).

```sh
target/release/glitu serve tests/fonctions/slides.md -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
```

Les captures sont reproductibles : `prefers-reduced-motion` coupe les animations, `Math.random`
est rejoué à l'identique (fond cristal), et chaque état est capturé quand l'écran ne bouge plus
(terminal en train de taper). Un état échoue au-delà de 0,5 % de pixels différents ; l'image des
différences est alors écrite à côté du rendu (`diff_*.png`).

Les schémas sont redessinés à chaque construction par le moteur skemo du dépôt voisin : une
évolution de skemo change donc les slides à schémas. Recapturer alors la
référence après avoir regardé le nouveau rendu :
`target/release/glitu oracle capture http://localhost:8123/ tests/oracle` (garder ce README).
