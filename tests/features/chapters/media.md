# Media

Captions, videos, focus

## A source under the visual

### Credit what you show

- A caption under any visual
- In Markdown, with its links @1

```yaml title="deployment.yml" caption="After the [Kubernetes documentation](https://kubernetes.io/docs/)" @1
apiVersion: apps/v1
kind: Deployment
spec:
  replicas: 3
```

## Focus on an image

### The office

- The screen @1
- The laptop, boxed in green @2
- A closer look at the plant @3

![The office](img/office.jpg "Illustration drawn for glitu"){focus="21 28 39 37 @1; 61 50 23 20 .box .green @2; 84 42 11 20 @3:magnify"}

## The rest in the shade {.image-right}

### Look here

- The laptop, the rest darkens @1

![](img/office.jpg "Caption laid on the image"){focus="61 50 23 20 .spot @1"}

## A video that pauses

### The game of life

- It plays, then stops at 1.5 s
- A click: on to 3 s @1
- A click: to the end @2

![Game of life](img/life.webm "Rendered by ffmpeg (life filter)"){muted pauses="0:01.5 0:03" focus="35 30 30 40 0:01.5-0:02"}

Note: The video stops by itself at 1.5 s, circled. [click] It resumes, stops at 3 s. [click] It plays to the end.
