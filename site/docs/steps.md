---
title: Steps and animations
description: Reveal content click by click with @n markers, choose entrance effects and highlights, set transitions and animate elements from one slide to the next.
order: 3
---

Everything in a slide can appear on a click: a bullet, a paragraph, a word, a code window, a
formula, or an element of a diagram. Clicks are numbered within each slide, and the same number
means the same click everywhere in the slide.

## Step markers

```markdown
- Number of replicas @1                ← appears at click 1
- The ReplicaSet creates the pods @+1   ← one click after the previous step: click 2
- Visible from click 2 to 3 @2-3        ← then disappears
- Rolling update @4:up                  ← click 4, entering from below
```

| Marker | Meaning |
|---|---|
| `@n` | Shown from click *n* |
| `@+n` | Shown *n* clicks after the previous step of the slide |
| `@n-m` | Shown from click *n* to click *m*, then hidden |
| `@0-2` | Shown from the start, hidden after click 2 |
| `@n:effect` | Shown at click *n* with an entrance effect |

The markers work on bullets, `###` titles, paragraphs, code windows (` ```yaml @1 `), formulas,
words (`[text]{@2}`), images and videos (`![](img/x.png){@2}`), zones of an image
(`{focus="10 20 30 30 @2"}`, see [Focus](/docs/layouts/#focus-on-part-of-an-image-or-a-video)) and
skemo diagram elements (`card#api API @2`). The pauses of a video take one click each (see
[Videos](/docs/layouts/#pausing-at-precise-times)). Keep the clicks of the text
aligned with those of the visual: bullet `@1` explains what appears at `@1` in the diagram.

## Effects

`@2:effect` chooses how an element appears: `up`, `down`, `left`, `right`, `grow`, `shrink`,
`zoom`, `red`, `green`, `blue` or `dim`. Any fragment class of reveal.js works too.

## Highlights

A highlight emphasizes an element that is already visible, for the duration of its clicks:

| Highlight | Effect |
|---|---|
| `@3-4:halo` | A glow, once |
| `@3-4:chase` | A light running around the element |
| `@3-4:magnify` | The view zooms on the element, then returns |

`halo` and `chase` fade out after three seconds at most. A highlight is added to the entrance of
the element:

```markdown
- Persistent storage @1 @3:halo
```

```yaml title="service.yml" @1 @4:chase
kind: Service
```

The colour is the theme's accent, or `--fx-color` if your `style.css` sets it.

## Blocks that replace each other

Several `###` titles with a step in the same slide show one block at a time:

```markdown
## Three ways to deploy

### Manually @1
- kubectl apply

### With Helm @2
- helm install

### With Argo CD @3
- A Git commit
```

## Transitions and backgrounds

Attributes on the `##` line set the transition and the background of a slide:

```markdown
## Results {transition="zoom" background="#123"}
## Our office {background="img/office.jpg"}
```

`class="x"` adds a class to the slide and `zoom="0.8"` scales its content. The theme sets the
default transition in its `glitu.yaml`.

### Zooming into a diagram

`transition="dive:api"` zooms from one slide to the next through a diagram: the diagram of the
previous slide grows around its element `api` (a skemo card) until it fills the stage, and the
diagram of the new slide comes out of it, while the text of the panel fades out, then the new one
in. `surface:api` zooms back out to the general diagram; going back with ← plays either the other
way.

```markdown
## The API in detail {transition="dive:api"}
## Back to the whole {transition="surface:api"}
```

Instead of an element, a zone of the picture: `dive:40 30 20 15` (x y width height, in %).

## Animated slides

`{.animate}` on two consecutive slides animates what they share: elements with the same text, the
same image or the same window title slide to their new place, and new elements fade in. To pair
elements whose text differs, name them: `[text]{#name}`.

```markdown
## Before {.animate}

- [Monolith]{#app}

## After {.animate}

- [Service A]{#app}
- Service B
```

## Animated code

A ` ````md magic-move ` block (four backticks) holds several versions of the same code, one per
click. The tokens that are kept slide to their new place, the others fade out or appear:

`````markdown
````md magic-move
```rust
let x = 1;
```
```rust
let x = compute();
```
````
`````

See [Code, diagrams and terminals](/docs/code/) for code windows and line highlighting.

## Automatic advance and timing

`data-autoslide` on a slide, or `autoSlide` (in milliseconds) in the theme's `glitu.yaml`,
advances the presentation on its own; <kbd>A</kbd> pauses and resumes it, and `loop: true`
restarts at the end. Planned durations, written in slide titles (`## Demo ⏳90`), are described in
[Presenting](/docs/presenting/#timing-and-rehearsals).
