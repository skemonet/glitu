---
title: ignoré (fichier inclus)
---

# Mises en page

Une slide, une forme

## Sommaire {.toc}

## Deux colonnes {.cols}

### Avant

- Un texte @1
- Visible des clics 2 à 3 @2-3
- Arrive par le bas @+1:up

::right::

### Après

Un paragraphe qui apparaît @2

```yaml title="config.yml" @3
replicas: 3
```

## Citation {.quote}

La simplicité est la sophistication suprême.

— Léonard de Vinci

## Chiffre clé {.fact transition="zoom"}

### 98 %

des incidents évités par l'automatisation

## Affirmation {.statement background="#1e3a8a"}

Le code est la [documentation]{.underline .cyan @1} qui ne [ment]{.circle @2} [jamais]{.highlight @3}.

## Centrée {.center class="sombre" zoom="0.8"}

### Du texte centré

[Encadré]{.box .green}, [barré]{.strike .red} et [qui apparaît]{@1}.

## Icônes et polices

### :mdi:github: Dépôt

- :mdi:rocket-launch: Rapide
- :mdi:shield-check: Sûr

## Mises en évidence

### Au clic

- Une lueur qui pulse @1 @2-3:halo
- Une lumière qui tourne autour @4-5:chase
- Un mot [agrandi]{@6-7:magnify} le temps de deux clics

```skemo
diagram(dir="down" lang="fr")
  card#api.system API                @1
    to(pod)
  card#pod.pod Pod                    @1 @8-9:halo
```

Note: Les mises en évidence s'ajoutent à l'apparition : halo, chase, magnify, sur un texte, un mot, une fenêtre de code ou un nœud de schéma.

## Titre {.intro}

Une slide titre au milieu de la présentation

glitu · 2026

## Intercalaire {.section}

Sans numéro ni changement de chapitre

## Image à droite {.image-right image="img/desk.jpg"}

### Le texte à gauche

- L'image couvre la moitié droite
- `{.image-left}` la met à gauche

## Colonnes avec en-tête {.cols}

Un en-tête sur toute la largeur, puis deux colonnes

::left::

- À gauche

::right::

- À droite

## Page web {.iframe url="https://glitu.net"}

## Merci {.end}

Des questions ?
