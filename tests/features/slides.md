---
lang: fr
title: Fonctions de glitu
subtitle: Formules, inclusions, notes au clic
author: glitu
font-title: Poppins
font-mono: Fira Code
global-top: "glitu · démonstration"
---

# Formules

En LaTeX, rendues au build

## Une formule dans le texte

### Énergie

- L'équivalence $E = mc^2$ @1
- Une somme $\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$ @2

```math @1
\int_{-\infty}^{+\infty} e^{-x^2}\,dx = \sqrt{\pi}
```

Note: Commencer par l'énergie. [click] Puis la somme de Gauss. [click] Enfin l'intégrale.

## Une formule en bloc

$$
\begin{pmatrix} a & b \\ c & d \end{pmatrix}
\begin{pmatrix} x \\ y \end{pmatrix} =
\begin{pmatrix} ax + by \\ cx + dy \end{pmatrix}
$$

<!-- include: chapters/layouts.md -->

<!-- include: chapters/code.md -->
