# glitu — presentations written in markdown, rendered by a Rust engine

> [!WARNING]
> **This code is 100% AI-generated.** It is provided "as is", without warranty of any kind; the
> repository owner accepts no responsibility for its use. Read it and test it before relying on it.
>
> **Status:** work in progress — the English translation of the documentation and of part of the
> code is not finished (see [TODO.md](TODO.md)).

## ▶ Demo: glitu, presented with glitu

[**`tests/features/slides.md`**](tests/features/slides.md) presents glitu itself: its publishing
modes, a frieze, the architecture diagram built click by click, its own source with the project
tree, and a terminal recording of `glitu build`; then a tour of every syntax.

```sh
rustup target add wasm32-unknown-unknown
cargo build --release
target/release/glitu serve tests/features/slides.md    # http://localhost:8000
```

Diagrams are drawn by [skemo](https://github.com/skemonet/skemo), a git dependency fetched by
Cargo.

**License:** dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

---

# glitu — des présentations écrites en markdown, rendues par un moteur Rust

Toute la présentation tient dans **`slides.md`** : le texte, les schémas (code [skemo](https://github.com/skemonet/skemo)),
le code montré à l'écran. Seuls les enregistrements de terminal (`casts/*.cast`) et les images
éventuelles sont des fichiers à part. Le rendu vient d'un **thème** (`themes/crystal/`) :
bandeau, panneau de texte à gauche, visuel à droite, backdrop « crystal », progression par chapitre.
Six autres thèmes partagent ce template avec un autre backdrop animé : `constellation`, `paper`
(paper découpé), `aurora`, `ascii` (ASCII ambre), `plan` (plan technique), `iso` (isométrique).

L'outil `glitu` (Rust) compile le markdown en une page statique ; le moteur de présentation,
écrit en Rust lui aussi et compilé en WebAssembly, remplace reveal.js dans le navigateur.
Aucune dépendance npm. Les règles d'écriture de reveal.js / reveal-md sont aussi acceptées.
Le moteur et les thèmes sont publiés par le mainteneur sur **static.glitu.net**, chacun à sa
version (`glitu/0.1.0/`, `themes/crystal/0.1.0/`).

```sh
rustup target add wasm32-unknown-unknown     # une fois ; skemo est récupéré par Cargo (dépendance git)
cargo build --release                         # produit target/release/glitu (moteurs wasm et skemo embarqués)

target/release/glitu serve                     # http://localhost:8000, recharge à chaque modification
target/release/glitu serve --allow 192.168.0.0/16   # aussi pour ces adresses (CIDR, répétable) : téléphone, tablette
target/release/glitu build -o dist             # site statique, à servir par n'importe quel serveur
target/release/glitu build --standalone -o prez.html   # une seule page, tout compris, sans réseau
target/release/glitu build --web https://static.glitu.net -o prez.html   # page légère (voir plus bas)
target/release/glitu export -o prez.pptx       # PowerPoint modifiable, notes comprises (voir « Exporter »)
target/release/glitu export -o prez.pdf        # PDF vectoriel (texte sélectionnable)
target/release/glitu export -o captures/       # une image PNG par slide
target/release/glitu unpack prez.html -o prez/ # ressort le markdown d'une page construite (voir « Rouvrir »)
```

Pendant la présentation (`?` affiche cette liste à l'écran) :

| Touche | Action |
|---|---|
| `→` Espace `N` | étape ou slide suivante, dans l'ordre de lecture |
| `←` Maj+Espace `P` | étape ou slide précédente |
| `Page↓` / `Page↑` | chapitre suivant / précédent en navigation 2D (sinon slide suivante / précédente) |
| `↓` / `↑` | dans le chapitre (`H` `J` `K` `L` : gauche, bas, haut, droite) |
| Début / Fin | première / dernière slide |
| `G` | aller à une slide : son numéro, `5.2`, ou son identifiant |
| Échap / `O` | vue d'ensemble : flèches pour choisir, Entrée ou clic pour y aller |
| `F` | plein écran |
| `S` | vue de l'orateur (voir « Présenter ») |
| `D` | dessiner sur la slide : 1 à 5 la couleur, `Z` annule, `X` efface, `D` ou Échap en sort |
| `B` `V` `.` | écran noir, puis reprise |
| `A` | suspendre ou reprendre le défilement automatique |
| Alt + clic (Ctrl + clic sous Linux) | zoomer sur un élément, puis revenir |

Sur un écran tactile, glisser d'un doigt change de slide ; à deux doigts, on zoome et on déplace
la vue. La molette avec `"mouseWheel": true` dans `glitu.json`.
Ajouter `?print-pdf` à l'adresse met une slide par page, toutes étapes affichées : imprimer
alors en PDF depuis le navigateur (ou `glitu export -o x.pdf`). Comme reveal.js : `data-autoplay` lance une vidéo ou un son à
l'arrivée sur la slide, `data-src` charge un média quand la slide approche, `data-autoslide`
(ou `"autoSlide"` en ms dans `glitu.json`) fait défiler seul, `"loop": true` reboucle,
`data-visibility="hidden"` masque une slide, `data-state="x"` pose la classe `x` sur la page.

## Écrire les slides

~~~markdown
---
title: Kubernetes                       ← la couverture
subtitle: Déployer et exposer une application
author: Ada Lovelace
date: Octobre 2026
theme: crystal                        ← ou crystal:0.1.0 ; sans version : celui du disque
---

# Ressources                            ← chapitre numéroté (01, 02…) ; `# Fin {-}` : sans numéro

Déployer, puis exposer                  ← sous-titre du chapitre

## Déploiement d'un applicatif          ← une slide : bandeau « RESSOURCES / Déploiement… »

### Deployment                          ← titre du panneau de gauche

- Nombre de réplicas @1                 ← apparaît au 1er clic
- ReplicaSet crée et supprime les pods @+1   ← un clic après le précédent (@2)
- Visible des clics 2 à 3 @2-3:up       ← puis disparaît ; arrive par le bas

```yaml title="deployment.yml" lines="8|9-11" @1     ← fenêtre de code, lignes surlignées au clic
…
```

Une énergie $E = mc^2$, un mot [clé]{.circle @3} entouré au clic 3.

Note: ce que l'orateur doit dire (touche S). [click] Au premier clic, ceci est surligné.
~~~

| Pour… | Écrire |
|---|---|
| un schéma | un bloc ` ```skemo ` ; ses marqueurs `@n` avancent avec les clics de la slide |
| une frise qui défile | un bloc skemo `diagram(layout="frise" …)` : la bande se centre sur la carte de chaque étape |
| un terminal | `![titre de la fenêtre](casts/demo.cast)` : une commande par clic |
| une image | `![](img/photo.jpg)` |
| des blocs qui se remplacent | plusieurs `### Titre @n` dans la slide |
| un schéma en pleine largeur | `## Sujet {.large}` : les puces passent en légende au-dessus |
| sans texte | une slide sans `###` ni puce : le visuel occupe toute la largeur |
| un autre bandeau | `## Sujet {kicker="Template"}` ; `{.compact}` resserre les puces |
| des emojis en grand | un bloc ` ```emoji `, un par ligne : ils se remplacent au clic (`😱 @1`) |
| le fichier montré | ` ```yaml file="roles/x/tasks/main.yml" ` : un explorateur liste les fichiers vus jusque-là |
| une navigation en 2D | `navigation: 2d` dans l'en-tête : un chapitre par colonne (← → et Espace suivent l'ordre de lecture, Page↓ Page↑ d'un chapitre à l'autre, ↑ ↓ dans le chapitre) |
| une formule | `$\frac{a}{b}$` dans le texte, `$$…$$` en bloc, un bloc ` ```math ` comme visuel (LaTeX, rendu en MathML au build) |
| un mot qui apparaît | `[texte]{@2}` ; `[texte]{.classe}` pose une classe |
| un mot marqué à la main | `[texte]{.circle @2}` : `.underline` `.circle` `.box` `.highlight` `.strike`, couleur `.red` `.green` `.blue` `.cyan` `.yellow` `.purple` ; tracé au clic, ou d'emblée sans `@n` |
| une icône | `:mdi:github:` : toute icône [Iconify](https://icon-sets.iconify.design) (`collection:nom`), téléchargée au build |
| une autre police | `font: Inter`, `font-title: Poppins`, `font-mono: Fira Code` dans l'en-tête (Google Fonts, téléchargées au build) |
| deux colonnes | `## Sujet {.cols}` ; une ligne `::right::` sépare la colonne de gauche de celle de droite (textes, code, schéma, image) |
| une citation | `## Sujet {.quote}` : le premier paragraphe en grand, le suivant pour l'auteur |
| un chiffre clé | `## Sujet {.fact}` : le titre `###` en très grand, le texte en légende |
| une affirmation | `## Sujet {.statement}` : le texte en grand, centré ; `{.center}` : simplement centré |
| un sommaire | `## Sommaire {.toc}` : les chapitres, en liens, générés |
| une transition, un backdrop | `## Sujet {transition="zoom" background="#123"}` (couleur, ou image : `background="img/x.jpg"`), `class="x"`, `zoom="0.8"` |
| des notes au fil des clics | dans `Note:`, `[click]` passe au clic suivant, `[click:3]` au clic 3 : la vue de l'orateur surligne le passage en cours |
| du code animé | un bloc ` ````md magic-move ` (quatre accents graves) contenant plusieurs blocs ` ``` ` : une version par clic, les mots gardés glissent à leur nouvelle place, les autres s'effacent ou apparaissent |
| du code annoté | en fin de ligne, en commentaire : `// [!code ++]` (ajoutée), `[!code --]` (retirée), `[!code focus]` (le reste s'estompe), `[!code highlight]`, `[!code error]`, `[!code warning]` ; l'annotation disparaît du code affiché |
| du code pris dans un fichier | une ligne `<<< src/main.rs` (chemin relatif au fichier markdown), `#nom` pour une région (`// #region nom` … `// #endregion`), `#L3-L8` pour des lignes ; puis, au besoin, le langage et les attributs du bloc (`title="…"`, `@2`) ; entre deux versions d'un code animé, une telle ligne en importe une |
| des slides animées | `{.animate}` sur deux slides qui se suivent : les éléments communs (même texte, même image, même titre de fenêtre, ou même `[texte]{#nom}`) glissent de l'une à l'autre, les nouveaux apparaissent en fondu |
| une slide titre | `## Titre {.cover}` ou `{.intro}` : le titre en grand, le 1er paragraphe en sous-titre, les suivants en signature |
| un intercalaire | `## Partie {.section}` (sans numéro ni changement de chapitre) ; `## Merci {.end}` pour finir |
| une image à côté du texte | `{.image-right}` ou `{.image-left}` : la première image de la slide (ou `image="img/x.jpg"`) couvre une moitié |
| une page web | `## Démo {.iframe url="https://…"}` : en pleine slide, ou à côté du texte (`{.iframe-left}`, `{.iframe-right}`) |
| sans marges | `## Sujet {.full}` : le visuel occupe toute la slide sous le bandeau |
| deux colonnes et un en-tête | dans `{.cols}`, ce qui précède une ligne `::left::` s'étend sur les deux colonnes |
| le logo du bandeau | `logo: img/logo.png` dans l'en-tête remplace celui du thème (une image de `img/` ou `public/`) ; `logo: none` le retire |
| un logo, une mention sur chaque slide | `global-top: "© ACME"` (devant le contenu, en bas à droite) ou `global-bottom:` (derrière, en bas à gauche) dans l'en-tête, en markdown : `![](img/logo.svg)` |
| plusieurs fichiers | une ligne `<!-- include: chapitres/deploiement.md -->` insère ce fichier (chemin relatif au fichier qui inclut ; son en-tête est ignoré, ses images restent relatives au fichier principal) |

Les étapes sont des entiers ; `@n` sur une puce, un titre `###`, un paragraphe, une fenêtre de
code, une formule ou dans un schéma skemo désigne le même clic. `@+n` compte depuis le clic
précédent de la slide, `@2-4` borne la présence (visible des clics 2 à 4), `@2:effet` choisit
l'apparition : `up` `down` `left` `right` `grow` `shrink` `zoom` `red` `green` `blue` `dim`,
ou toute classe de fragment de reveal.js. Une **mise en évidence** s'ajoute à l'apparition, le
temps de ses clics : `@3-4:halo` (une lueur, une fois), `@3-4:chase` (une lumière qui fait
le tour), tous deux éteints au bout de 3 s au plus, et `@3-4:magnify` (la vue s'agrandit autour de
l'élément, puis revient) ; sur une puce
(`- Point @1 @3:halo`), un paragraphe, un mot (`[clé]{@2-3:magnify}`), une fenêtre de code
(` ```yaml @1 @4:chase `) ou un nœud de schéma skemo (`card#pg Base @1:up @3-4:halo`).
Couleur : `--fx-color`, sinon l'accent du thème. Les espaces avant `: ; ! ? »` deviennent insécables.
Une ligne faite d'un seul `=` (ou `-`) souligne le texte du dessus en titre, règle du markdown :
dans une formule sur plusieurs lignes, garder le `=` sur la même ligne qu'un terme.

Le code est coloré au build : YAML, HTML/XML, et les langages à la C (Rust, Python, Bash, Go,
Java, JavaScript, TypeScript, PHP…) ; les autres restent en texte brut.

## Présenter

`S` ouvre la **vue de l'orateur**, une fenêtre à part : la slide telle que le public la voit,
l'étape qui vient, les notes (le passage du clic en cours surligné, voir `[click]`), l'heure et
un chronomètre (un clic le met en pause ; « Remettre à zéro »). Les touches de la présentation
y fonctionnent : on peut piloter depuis cette fenêtre.

Sous `glitu serve`, **les fenêtres ouvertes restent ensemble** : changer de slide sur l'une
déplace les autres (projecteur, portable, tablette). Avec `--allow 192.168.0.0/16`, ouvrir sur
un téléphone l'adresse affichée suivie de `?remote` donne une **télécommande** : la slide, deux
grands boutons et les notes ; elle rejoint la présentation où elle en est (`?follow` fait de
même sans les boutons).

`D` permet de **dessiner** sur la slide (souris, doigt, stylet). Les traits restent avec leur
slide et sont gardés par le navigateur d'une séance à l'autre.

`?view=scroll` à la fin de l'adresse donne la **vue défilante** : les slides les unes sous les
autres, à la largeur de la fenêtre, toutes étapes affichées, à lire comme une page web. Un
téléphone (fenêtre de moins de 435 px) l'a d'office, sauf en télécommande ; tourné en paysage, il
repasse en présentation (la page se recharge, à la même slide). Le doigt, la molette et les
flèches y font défiler la page.

## Exporter

```sh
glitu export [slides.md] [-o presentation.pptx | x.pdf | dossier/] [--format pptx|pdf|png]
             [--range 1-3,5] [--clicks] [--images]
```

Chromium (headless ; variable `CHROMIUM` pour un autre) rejoue la présentation. Chaque slide
est prise une fois toutes ses étapes passées, ou à chaque clic avec `--clicks` ; `--range`
garde certaines slides (numéros dans l'ordre de lecture).

- **PowerPoint** : chaque élément devient un objet PowerPoint **modifiable** — zones de texte
  (polices, couleurs, gras, liens, puces, retours à la ligne d'origine), formes (backdrops, dégradés,
  bordures, coins arrondis, ombres), schémas skemo en groupes de formes et de tracés, images.
  Seuls le backdrop animé du thème et ce qui ne se traduit pas (canvas, vidéo, formules, élément
  tourné ou coupé par un cadre) restent des images. Les notes vont dans la page de notes.
  Les polices du thème (Lato, Inter, JetBrains Mono pour « crystal ») doivent être installées
  là où l'on ouvre le fichier. `--images` revient à une capture par slide (fidèle, non modifiable).
- **PDF** : la page imprimée par Chromium, en vectoriel (texte sélectionnable) ; avec
  `--clicks`, une page par clic (captures) et un sommaire des titres de slides.
- **PNG** : `NN.png` par slide (`NN-c.png` par clic avec `--clicks`) dans le dossier donné.

## Rouvrir une présentation

Chaque page construite garde une copie de son markdown, fichier par fichier (inclusions et
code importé compris), dans un `<script type="application/json" id="glitu-source">` que le
navigateur n'exécute pas. `glitu unpack page.html -o dossier/` (ou une adresse `https://…`) les
ressort, pour modifier la présentation et la reconstruire ; `--force` remplace des fichiers
existants. Les images et les terminaux ne sont pas dans la copie : les remettre à côté.
`source: false` dans l'en-tête construit la page sans copie.

## Une seule page

`--standalone` écrit tout dans la page : moteur, thème, polices, images. Elle s'ouvre par
double-clic et ne demande aucun réseau (environ 2 Mo) : c'est celle à emporter pour présenter.

`--web <url>` ne garde dans la page que la présentation (texte, schémas, terminaux, images) ; le
moteur et le thème viennent de `<url>` : static.glitu.net, ou toute adresse où l'on a publié ce
que produit `glitu-site static` (GitHub Pages convient : il autorise les pages locales à les charger).
La page vise des versions exactes, `glitu/<version du binaire>/` et `themes/<nom>/<version du
thème>/`, et vérifie chaque fichier par son empreinte SHA-384 : un fichier republié différemment
fait refuser la page plutôt que la casser en silence. Un thème tiers (`theme: ./chemin`), qui
n'est pas publié, reste écrit dans la page. Il faut le réseau pour présenter.

## Versions

Le front matter peut fixer les versions attendues ; sans version, c'est la dernière disponible,
c'est-à-dire celle du binaire et du dossier `themes/` utilisés :

```yaml
glitu: 0.1.0               # version du moteur ; une autre est refusée à la construction
theme: crystal:0.1.0     # thème officiel à cette version (glitu.json, "version") ; « crystal » seul : le dernier
theme: ./themes-maison/x   # thème tiers, chemin relatif à la présentation
```

## Publier static.glitu.net (mainteneur)

`cargo run --release -p glitu-site -- static [-o site/static]` écrit `site/static/glitu/<version>/`
(le moteur, à la version du dépôt) et `site/static/themes/<nom>/<version>/` (la `"version"` du
`glitu.json` de chaque thème, obligatoire), à publier tel quel sur static.glitu.net (rsync, scp…).
Les versions déjà présentes dans `site/static/` restent : on publie par ajout. Seul le mainteneur
publie.

## Site glitu.net (mainteneur)

Le site est un sous-projet à part, `site/` (crate `glitu-site`, ni distribué ni publié) : ses
pages (`site/pages/`) et l'outil qui l'assemble. `cargo run --release -p glitu-site` écrit dans
`site/dist/` la page d'accueil, la documentation (les sections de ce README destinées aux
utilisateurs : une seule source) et la démonstration (`tests/features`). À servir tel quel.

## Schémas

Les blocs skemo sont rendus à la volée, à chaque construction, par le moteur skemo lui-même :
`glitu` est lié à la bibliothèque `skemo-engine` de [skemo](https://github.com/skemonet/skemo)
(dépendance git, épinglée sur une révision), avec la feuille `skemo.css` du thème, complétée par celle de la présentation : un
`skemo.css` à côté du markdown y ajoute ses propres types (`.pod { --c: #2563eb; --label: "POD"; }`),
gardé avec le markdown dans la page construite. Pas de binaire externe ni de cache : modifier un
schéma dans `slides.md` le redessine aussitôt (`glitu serve` recharge la page). Un avertissement
de skemo fait échouer la construction, avec la ligne fautive dans le bloc.

La taille des textes pour la projection vient du thème : `"skemo": { "fontScale": 1.35 }` dans
`glitu.json` (option `--font-scale` de skemo, qui en tient compte dans sa mise en page). Un bloc
peut la changer : ` ```skemo font-scale="2" `. `skemo.css` ne règle que les couleurs et les
étiquettes des types. Viser au moins 16 px à l'écran : un schéma en pleine largeur tient dans
~1150 × 590 px. `diagram(fit="2:1")` fait échouer le build si les proportions s'écartent de plus
de 15 % de la cible (`skemo fit` propose des réglages ; vérifier qu'ils n'y perdent pas en taille).

```pug
card#api.system Kubernetes API   @2:up        //- apparaît au clic 2, en montant
  to(k1 k2).dashed ordres         @4           //- flèches tracées au clic 4
card#note.note Ports fermés       @1-2         //- visible aux clics 1 et 2, puis s'efface
card#p1.pod front                 @3:focus     //- mis en valeur au clic 3
card#p2.pod(tag=":8080 @1") back               //- la pastille apparaît au clic 1
```

## Terminal

Un enregistrement asciicast (v2 ou v3) se rejoue segment par segment : le lecteur tape les
commandes, s'arrête sur chaque marker et attend le clic suivant ; un clic pendant la frappe
termine le segment en cours. L'émulation du terminal est faite par `avt`, celui d'asciinema.
Pour un vrai enregistrement : `asciinema rec` avec `rec.add_marker_key`, ou ajouter des lignes
`[temps, "m", "label"]` au fichier. `tools/build_cast.py` fabrique celui de la démo.

## Thème

Un thème est un dossier `themes/<nom>/` : ses feuilles et polices, `glitu.json` (feuilles à charger,
taille des slides, marge, agrandissement maximal `maxScale` — 2 par défaut, 8 pour les thèmes du
template, qui remplissent ainsi un grand écran —, centrage, flèches, progression, numéro, transition), et selon le thème
`logo.svg`, `skemo.css`. `glitu.json` porte aussi sa `"version"` et, pour un thème tiers, le
`"name"` que le moteur reconnaît. Il est cherché à côté du markdown, puis dans les dossiers parents.

Les thèmes de ce dépôt, sauf « black », reposent sur un **template commun** (`"template": true`) :
sa feuille, `glitu/template.css`, publiée avec le moteur, porte toute la mise en page (bandeau,
panneau, scène, couverture, chapitres, progression, mises en page) et ne se règle que par des
variables CSS (palette, polices, voiles, `--tilt` : inclinaison de la séparation panneau/scène,
2° dans le sens horaire) ; le `theme.css` d'un thème se limite à ces variables. Sa partie
dynamique (bandeau, progression, backdrop) est `crates/glitu-web/src/themes/template.rs`, et le backdrop
se choisit par `"backdrop"` : `crystal`, ou une scène de `crates/glitu-web/src/themes/backdrops/`
(`constellation`, `paper`, `aurora`, `ascii`, `plan`, `iso`). Il habille le balisage que
produit le compilateur :

| Layout | Balisage |
|---|---|
| Couverture | `.cover` : `.eyebrow`, `h1`, `.subtitle`, `.byline` |
| Chapitre | `.chapter` : `.num`, `h2`, `p` |
| Slide | `section[data-header="Rubrique\|Sujet"]` |
| Texte | `.panel` : `h2`, `ul` ; `.panel-steps` pour des blocs qui se remplacent |
| Visuel | `.stage` (`.full` sans texte) : `.diagram`, `.window` (code ou `.cast-stage`), `.pan-track` |
| Légende | `.stage.full > .caption` : `h2` + `.caption-steps` |
| Mises en page | `section.layout-cols > .cols > .col` ; `section.layout-quote` (`fact`, `statement`, `center`, `toc`) `> .layout-body` |
| Formule | `.formula` (bloc ` ```math `), `math` (MathML) |

Les règles communes à tous les thèmes (étapes `.until`, marques `.mark`, dessin `svg.drawing`,
télécommande `.remote-pad`, formules `glitu/math.css`) sont dans la feuille du moteur ; un thème
peut les surcharger.

Le backdrop est unique pour toute la présentation (`.tpl-backdrop`, sous les slides) : une slide
demande seulement qu'il tourne (`data-backdrop="animated"`, couverture et chapitres) ou qu'il se fige
(`data-backdrop="still"`) ; il ralentit ou repart sans changer d'image. Les nouveaux backdrops s'estompent
en plus côté texte, progressivement : entiers sous la couverture et les chapitres, à moitié à
côté d'un panneau, effacés sous une slide de texte pleine largeur. Le compilateur marque
chaque slide `data-layout="full"` ou `"panel"`, et le moteur pose sur la slide qui arrive la mise
en page de la précédente (`data-from-layout`) : plein écran → panneau, le contenu s'efface puis
le panneau glisse depuis la gauche ; panneau → plein écran, l'un s'efface puis l'autre apparaît.

## Règles d'écriture de reveal.js / reveal-md

Un markdown écrit pour reveal-md (slides séparées par `---`, verticales par `----`, `Note:`,
`<!-- .slide: … -->`, `<!-- .element: … -->`, code ` ```php [1|3-5] `) est reconnu d'office et rendu
avec le thème « black » de reveal-md. Le moteur produit le même DOM que reveal.js : slides
verticales, backdrops (`data-background…`), flèches, barre de progression, numéro de slide, transitions,
centrage. `tests/reveal-md/` vérifie ce rendu contre celui de reveal-md lui-même.

## Oracle

`tests/oracle/` garde une capture de chaque état (slide × clic) du rendu de référence, celui
de reveal.js écrit à la main. Le moteur Rust le reproduit au pixel près :

```sh
target/release/glitu serve -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
```

Voir `tests/oracle/README.md`.

## Arborescence

```text
themes/crystal/          le thème de ce projet : palette, polices, logo, skemo.css
themes/constellation/ …    les autres thèmes à template (paper, aurora, ascii, plan, iso)
themes/black/              le thème par défaut de reveal-md (reveal.js, MIT)
crates/glitu/               l'outil : construction du site, page unique, serveur, export
                           (PowerPoint, PDF, PNG), inclusions, icônes et polices, oracle
crates/glitu-core/          le compilateur : markdown → HTML, coloration, formules, schémas skemo
site/                      le site glitu.net et static.glitu.net (mainteneur) : pages, assemblage,
                           moteur et thèmes publiés (site/dist/, site/static/, non versionnés)
crates/glitu-web/           le moteur dans le navigateur (wasm) : navigation, étapes, thème,
                           schémas, terminal, défilement du code, code animé, slides
                           animées, vue défilante, vue de l'orateur, télécommande, dessin
tests/oracle/              captures de référence de tests/features (thème formation)
tests/reveal-md/           exemples markdown de reveal.js et reveal-md, rendu de référence
tests/features/           démonstration des fonctions : formules, inclusions, mises en page,
                           marques, icônes, polices, notes au clic, code animé, annoté et
                           importé, slides animées (aussi la démo du site)
crates/glitu/assets/       styles du moteur (glitu.css ; reveal.css minifié dans vendor/), relevé
                           de l'écran pour l'export PowerPoint (export.js), police des formules
tools/                     asciicast.py, pour écrire des enregistrements de terminal
docs/reference/            version Pandoc précédente, prototype terminal
```
