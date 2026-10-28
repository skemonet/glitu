---
title: reveal.js — exemples markdown
---

# Markdown Demo

Exemples officiels de reveal.js (`examples/markdown.html`, `markdown.md`)
et démo de reveal-md, aux règles d'écriture de reveal-md.

---

## External 1.1

Content 1.1

Note: This will only appear in the speaker notes window.

----

## External 1.2

Content 1.2

---

## External 2

Content 2.1

---

## External 3.1

Content 3.1

----

## External 3.2

Content 3.2

----

## External 3.3 (Image)

![External Image](img/symbol.png)

---

## Demo 2

Slide 1.1

----

## Demo 2

Slide 1.2

---

<!-- .slide: data-background="#000000" -->
## Slide attributes

---

<!-- .slide: data-background-color="#2a5ba8" -->
## Background color

---

## Element attributes

- Item 1 <!-- .element: class="fragment" data-fragment-index="2" -->
- Item 2 <!-- .element: class="fragment" data-fragment-index="1" -->

---

```php [1|3-5]
public function foo()
{
    $foo = array(
        'bar' => 'bar'
    )
}
```

---

## echo.c

```c [287: 2|4,6]
/* All of the options in this arg are valid, so handle them. */
p = arg + 1;
do {
    if (*p == 'n')
        nflag = 0;
    if (*p == 'e')
        eflag = '\\';
} while (*++p);
```

[source](https://git.busybox.net/busybox/tree/coreutils/echo.c?h=1_36_stable#n287)

---

![Sample image](img/symbol.png)

---

Foo

![cat](img/cat.jpg)

Note: test note

---

Bar

----

Sub Bar

---

The End.
