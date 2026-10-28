# Compatibilité avec les règles d'écriture de reveal.js / reveal-md

`demo.md` réunit les exemples markdown officiels de reveal.js (`examples/markdown.html`,
`examples/markdown.md`) et la démo de reveal-md (`demo/a.md`), écrits aux règles de reveal-md :

| Règle | Exemple |
|---|---|
| slides séparées par `---`, slides verticales par `----` | |
| notes de l'orateur | `Note: …` |
| attributs de slide | `<!-- .slide: data-background="#000000" -->` |
| attributs d'élément | `- Item <!-- .element: class="fragment" data-fragment-index="2" -->` |
| lignes surlignées par étapes, numérotation décalée | ` ```php [1\|3-5] `, ` ```c [287: 2\|4,6] ` |

`reference/` est le rendu de **reveal-md 6.1.4** (reveal.js 5.1.0) pour ce fichier, une capture par
état, en parcourant la présentation avec `Page↓`. Il a été produit hors du projet :
`npx reveal-md demo.md --static out`, puis `deck oracle capture`.

`deck` lit ces règles d'office (un séparateur `---` suffit à les reconnaître) et applique le
thème « black » de reveal-md (`themes/black/`). Le moteur Rust reproduit la référence :

```sh
target/release/deck serve tests/reveal-md/demo.md -p 8124 &
target/release/deck oracle check http://localhost:8124/ tests/reveal-md/reference
```

Non pris en charge : les formules (`$$…$$`, plugin math), les diagrammes mermaid, les options
`revealOptions` de l'en-tête, la vue d'ensemble (touche O) et l'export PDF.
