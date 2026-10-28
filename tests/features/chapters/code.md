# Code

Animé, annoté

## Code animé

````md magic-move title="total.rs"
```rust
fn total(v: &Vec<i32>) -> i32 {
    let mut s = 0;
    for x in v {
        s += x;
    }
    s
}
```
```rust
fn total(v: &[i32]) -> i32 {
    let mut s = 0;
    for x in v {
        s += x;
    }
    s
}
```
```rust
fn total(v: &[i32]) -> i32 {
    v.iter().sum()
}
```
````

Note: Une version par clic : les mots gardés glissent à leur nouvelle place.

## Code annoté

```python title="app.py"
def bonjour(nom):
    print("Bonjour " + nom)  # [!code --]
    print(f"Bonjour {nom}")  # [!code ++]
    return nom  # [!code focus]
```

## Code importé

<<< ../src/total.rs#somme title="src/total.rs"

Note: Le code vient du fichier, région « somme » : il reste à jour.

## Slides animées {.animate .center}

[Glisse ici]{#etiquette .highlight}

- Un premier point

## Slides animées {.animate .center}

- Un premier point
- Un deuxième, qui arrive en fondu
- Un troisième

[Glisse ici]{#etiquette .highlight}

Note: Les éléments communs aux deux slides glissent ; les nouveaux apparaissent.
