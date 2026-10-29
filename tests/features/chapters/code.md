# Code

Animated, annotated

## Animated code

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

Note: One version per click: the words kept slide to their new place.

## Annotated code

```python title="app.py"
def hello(name):
    print("Hello " + name)  # [!code --]
    print(f"Hello {name}")  # [!code ++]
    return name  # [!code focus]
```

## Imported code

<<< ../src/total.rs#sum title="src/total.rs"

Note: The code comes from the file, region "sum": it stays up to date.

## Animated slides {.animate .center}

[Slides here]{#label .highlight}

- A first point

## Animated slides {.animate .center}

- A first point
- A second one, fading in
- A third one

[Slides here]{#label .highlight}

Note: The elements shared by the two slides slide; the new ones appear.
