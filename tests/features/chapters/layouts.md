---
title: ignored (included file)
---

# Layouts

One slide, one shape

## Contents {.toc}

## Two columns {.cols}

### Before

- A text @1
- Visible from click 2 to 3 @2-3
- Comes in from below @+1:up

::right::

### After

A paragraph that appears @2

```yaml title="config.yml" @3
replicas: 3
```

## Quote {.quote}

Simplicity is the ultimate sophistication.

— Leonardo da Vinci

## Key figure {.fact transition="zoom"}

### 98 %

of incidents avoided by automation

## Statement {.statement background="#1e3a8a"}

Code is the [documentation]{.underline .cyan @1} that [never]{.circle @2} [lies]{.highlight @3}.

## Centred {.center class="dark" zoom="0.8"}

### Centred text

[Boxed]{.box .green}, [struck out]{.strike .red} and [appearing]{@1}.

## Icons and fonts

### :mdi:github: Repository

- :mdi:rocket-launch: Fast
- :mdi:shield-check: Safe

## Highlights

### On click

- A glow that pulses @1 @2-3:halo
- A light that runs around @4-5:chase
- A word [magnified]{@6-7:magnify} for two clicks

```skemo
diagram(dir="down")
  card#api.system API                @1
    to(pod)
  card#pod.pod Pod                    @1 @8-9:halo
```

Note: Highlights add to the appearance: halo, chase, magnify, on a text, a word, a code window or a diagram node.

## Title {.intro}

A title slide in the middle of the presentation

glitu · 2026

## Interlude {.section}

Without a number nor a change of chapter

## Image on the right {.image-right image="img/desk.jpg"}

### The text on the left

- The image covers the right half
- `{.image-left}` puts it on the left

## Columns with a header {.cols}

A header across the full width, then two columns

::left::

- On the left

::right::

- On the right

## Web page {.iframe url="page.html"}
