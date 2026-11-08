//! The keyboard's layout, read from the keymap that the Wayland compositor sends (XKB's text
//! format): which symbol each key gives, without modifiers. That is all the player needs (a
//! few named keys and letters), without the xkbcommon library: an AZERTY keyboard gives `q` on
//! the key where a QWERTY one gives `a`, as the compositor says.

use std::collections::HashMap;

use crate::show::Command;

/// The symbol of each key (by Wayland key code: the evdev code), first group, first level
#[derive(Debug, Default)]
pub struct Keymap {
    symbols: HashMap<u32, String>,
}

/// The text between `{` after `name` and its closing `}`
fn section<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(name)?;
    let open = start + text[start..].find('{')? + 1;
    let mut depth = 1;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open..open + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The statements of a section, split at `;` outside the braces
fn statements(body: &str) -> Vec<&str> {
    let (mut out, mut depth, mut from) = (Vec::new(), 0, 0);
    for (i, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ';' if depth == 0 => {
                out.push(body[from..i].trim());
                from = i + 1;
            }
            _ => {}
        }
    }
    out
}

/// `<NAME>` at the start of `s`, and what follows
fn key_name(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start().strip_prefix('<')?;
    let end = s.find('>')?;
    Some((&s[..end], &s[end + 1..]))
}

/// The symbols of the first group in the body of a `key <NAME> { … }`: either a bare list
/// `[ a, A ]`, or `symbols[Group1]= [ a, A ]`
fn first_group(body: &str) -> Option<Vec<&str>> {
    let mut rest = body;
    while let Some(open) = rest.find('[') {
        let before = rest[..open].trim_end();
        let named = before.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_');
        let close = open + rest[open..].find(']')?;
        let inside = &rest[open + 1..close];
        if !named {
            // a bare list: the first group's symbols
            return Some(inside.split(',').map(str::trim).collect());
        }
        // `symbols[Group1]` (or `[1]`) then `= [ … ]`; another field (`actions[…]`) is skipped
        let field = before.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).next().unwrap_or("");
        let after = rest[close + 1..].trim_start().strip_prefix('=')?.trim_start();
        let list = after.strip_prefix('[')?;
        let end = list.find(']')?;
        if field == "symbols" && matches!(inside.trim(), "Group1" | "group1" | "1") {
            return Some(list[..end].split(',').map(str::trim).collect());
        }
        rest = &list[end + 1..];
    }
    None
}

impl Keymap {
    /// Reads the compositor's keymap; keys it does not understand are left out
    pub fn parse(text: &str) -> Keymap {
        // the names of the keys and their codes (X11 codes: the evdev code + 8)
        let mut codes: HashMap<&str, u32> = HashMap::new();
        let mut aliases: Vec<(&str, &str)> = Vec::new();
        for st in statements(section(text, "xkb_keycodes").unwrap_or("")) {
            if let Some(alias) = st.strip_prefix("alias") {
                let Some((name, rest)) = key_name(alias) else { continue };
                let Some((target, _)) = rest.trim_start().strip_prefix('=').and_then(key_name) else { continue };
                aliases.push((name, target));
            } else if let Some((name, rest)) = key_name(st)
                && let Some(code) = rest.trim_start().strip_prefix('=').and_then(|v| v.trim().parse().ok())
            {
                codes.insert(name, code);
            }
        }
        for (name, target) in aliases {
            if let Some(&code) = codes.get(target) {
                codes.entry(name).or_insert(code);
            }
        }
        let mut symbols = HashMap::new();
        for st in statements(section(text, "xkb_symbols").unwrap_or("")) {
            let Some(key) = st.strip_prefix("key") else { continue };
            let Some((name, rest)) = key_name(key) else { continue };
            let (Some(&code), Some(body)) = (codes.get(name), section(rest, "")) else { continue };
            let first = first_group(body).and_then(|g| g.first().map(|s| s.to_string()));
            if let Some(sym) = first.filter(|s| !s.is_empty() && s != "NoSymbol")
                && code >= 8
            {
                symbols.insert(code - 8, sym);
            }
        }
        Keymap { symbols }
    }

    /// The symbol of a key (Wayland's key code), by its XKB name: `Right`, `space`, `q`…
    pub fn symbol(&self, key: u32) -> Option<&str> {
        self.symbols.get(&key).map(String::as_str)
    }
}

/// What the key of XKB symbol `sym` does
pub fn command(sym: &str) -> Option<Command> {
    Some(match sym {
        "Right" | "Down" | "space" | "Return" | "KP_Enter" | "Next" | "Page_Down" => Command::Next,
        "Left" | "Up" | "BackSpace" | "Prior" | "Page_Up" => Command::Previous,
        "Home" => Command::First,
        "End" => Command::Last,
        "F11" => Command::Fullscreen,
        "Escape" => Command::LeaveFullscreen,
        "plus" | "KP_Add" | "equal" => Command::Bigger,
        "minus" | "KP_Subtract" => Command::Smaller,
        // a letter: its name is itself
        s if s.chars().count() == 1 => return Command::of_char(s),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keymap(name: &str) -> Keymap {
        let path = format!("{}/tests/keymaps/{name}.xkb", env!("CARGO_MANIFEST_DIR"));
        Keymap::parse(&std::fs::read_to_string(path).unwrap())
    }

    /// evdev codes of some keys: the second row's first letter, the third row's first letter,
    /// the first row's sixth key, the one before Backspace, the right arrow, F11, page up
    const AD01: u32 = 16;
    const AC01: u32 = 30;
    const AE06: u32 = 7;
    const AE12: u32 = 13;
    const AD12: u32 = 27;
    const AB10: u32 = 53;

    #[test]
    fn the_keys_of_a_french_keyboard() {
        let k = keymap("fr");
        assert_eq!((k.symbol(AD01), k.symbol(AC01)), (Some("a"), Some("q")));
        assert_eq!((k.symbol(AE06), k.symbol(AE12)), (Some("minus"), Some("equal")));
        assert_eq!((k.symbol(106), k.symbol(87), k.symbol(104)), (Some("Right"), Some("F11"), Some("Prior")));
        assert_eq!((k.symbol(57), k.symbol(28), k.symbol(1)), (Some("space"), Some("Return"), Some("Escape")));
        // `q` quits where the keyboard says q, not where a QWERTY one does
        let of = |key| k.symbol(key).and_then(command);
        assert_eq!((of(AC01), of(AD01)), (Some(Command::Quit), None));
        assert_eq!((of(AE06), of(AE12), of(106)), (Some(Command::Smaller), Some(Command::Bigger), Some(Command::Next)));
    }

    #[test]
    fn the_keys_of_other_layouts() {
        let us = keymap("us");
        assert_eq!((us.symbol(AD01), us.symbol(AC01), us.symbol(AE12)), (Some("q"), Some("a"), Some("equal")));
        let de = keymap("de");
        // German: `+` on its own key, `-` near the right shift, z and y swapped
        assert_eq!((de.symbol(AD12), de.symbol(AB10)), (Some("plus"), Some("minus")));
        assert_eq!((de.symbol(21), de.symbol(44)), (Some("z"), Some("y")));
    }

    #[test]
    fn the_two_ways_of_writing_a_key() {
        let text = "xkb_keymap { xkb_keycodes \"x\" { <A> = 38; <B> = 39; <C> = 40; alias <D> = <A>; }; \
                    xkb_symbols \"x\" { key <A> { [ q, Q ] }; \
                    key <B> { type= \"TWO\", actions[Group1]= [ NoAction() ], symbols[Group1]= [ s, S ] }; \
                    key <C> { [ NoSymbol ] }; }; };";
        let k = Keymap::parse(text);
        assert_eq!((k.symbol(30), k.symbol(31), k.symbol(32)), (Some("q"), Some("s"), None));
        assert!(Keymap::parse("garbage {").symbols.is_empty());
    }
}
