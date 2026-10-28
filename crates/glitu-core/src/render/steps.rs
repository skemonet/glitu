//! Clicks: how an element appears (`@n` → reveal.js fragments), its highlights (`data-fx`), the
//! blocks that replace one another, and the speaker notes split by click.

use std::collections::BTreeSet;

use crate::html::inline;
use crate::markdown::Stepped;

/// `<tag>` of a text that appears at its click, with its effect, and disappears after its end
pub(super) fn stepped_html(tag: &str, it: &Stepped) -> String {
    let a = appear(it);
    let class = if a.class.is_empty() { String::new() } else { format!(" class=\"{}\"", a.class.trim_start()) };
    format!("<{tag}{class}{}{}>{}{}</{tag}>", a.attrs, a.fx, inline(&it.text), a.end)
}

/// Appearance effects (`@2:up`): short names, or a reveal.js fragment class as is
pub fn effect_class(effect: &str) -> &str {
    match effect {
        "up" => "fade-up",
        "down" => "fade-down",
        "left" => "fade-left",
        "right" => "fade-right",
        "zoom" => "zoom-in",
        "red" => "highlight-red",
        "green" => "highlight-green",
        "blue" => "highlight-blue",
        "dim" => "semi-fade-out",
        other => other,
    }
}

/// How an element appears: classes to add (` fragment fade-up`), index attribute, and the mark
/// to put at the end of its content if it disappears after its last click: an empty fragment
/// which, once visible, hides its parent (`.until`, see glitu.css)
pub struct Appear {
    pub class: String,
    pub attrs: String,
    pub end: String,
    /// Highlights: ` data-fx="halo:3-4 magnify:5"`, which the engine applies at the click
    pub fx: String,
}

pub fn appear(it: &Stepped) -> Appear {
    let effect = it.effect.as_deref().map(|e| format!(" {}", effect_class(e))).unwrap_or_default();
    let (class, attrs) = match it.step {
        0 => (effect, String::new()),
        n => (format!(" fragment{effect}"), format!(" data-fragment-index=\"{n}\"")),
    };
    let end = it
        .until
        .map(|u| format!("<span class=\"fragment until\" data-fragment-index=\"{}\"></span>", u + 1))
        .unwrap_or_default();
    Appear { class, attrs, end, fx: fx_attr(&it.marks) }
}

/// ` data-fx="halo:3-4 magnify:5"`: effect, first click and last one (none: until the end)
pub(super) fn fx_attr(marks: &[Stepped]) -> String {
    let fx: Vec<String> = marks
        .iter()
        .filter_map(|m| {
            let range = m.until.map(|u| format!("{}-{u}", m.step)).unwrap_or_else(|| m.step.to_string());
            Some(format!("{}:{range}", m.effect.as_deref()?))
        })
        .collect();
    if fx.is_empty() { String::new() } else { format!(" data-fx=\"{}\"", fx.join(" ")) }
}

/// The values of every `name="…"` attribute in `html`
fn attr_values<'a>(html: &'a str, name: &str) -> impl Iterator<Item = &'a str> {
    let prefix = format!("{name}=\"");
    html.split(prefix.as_str()).skip(1).map(|part| part.split('"').next().unwrap_or("")).collect::<Vec<_>>().into_iter()
}

/// One invisible fragment of class `class` per click (click 0 left out): the slide gets these
/// clicks even when nothing else appears at them
fn fragment_spans(class: &str, steps: BTreeSet<u32>) -> String {
    steps
        .into_iter()
        .filter(|&n| n > 0)
        .map(|n| format!("<span class=\"fragment {class}\" data-fragment-index=\"{n}\"></span>"))
        .collect()
}

/// An invisible fragment at the start and after the end of each highlight (`data-fx`)
pub(super) fn fx_steps(html: &str) -> String {
    let mut steps = BTreeSet::new();
    for value in attr_values(html, "data-fx") {
        for fx in value.split_whitespace() {
            let range = fx.split_once(':').map_or("", |(_, r)| r);
            let (a, b) = range.split_once('-').map_or((range, None), |(a, b)| (a, Some(b)));
            steps.extend(a.parse::<u32>().ok());
            steps.extend(b.and_then(|b| b.parse::<u32>().ok()).map(|b| b + 1));
        }
    }
    fragment_spans("fx-step", steps)
}

/// The steps of a skemo SVG: appearances (`data-step`), disappearances (`data-until` + 1), start
/// and end + 1 of the emphases (`data-focus="4 7-8"`)
pub(super) fn skemo_steps(html: &str) -> String {
    let mut steps = BTreeSet::new();
    for (name, after) in [("data-step", 0), ("data-until", 1)] {
        steps.extend(attr_values(html, name).filter_map(|v| v.parse::<u32>().ok()).map(|n| n + after));
    }
    for value in attr_values(html, "data-focus") {
        for range in value.split_whitespace() {
            let (a, b) = range.split_once('-').unwrap_or((range, range));
            if let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) {
                steps.insert(a);
                steps.insert(b + 1);
            }
        }
    }
    fragment_spans("sk-step", steps)
}

/// Blocks that replace one another at the click: the first fades out, the middle ones appear
/// then fade out
pub(super) fn replacing(items: &[Stepped], tag: &str, mut render: impl FnMut(&Stepped) -> String) -> String {
    let mut out = String::new();
    for (k, it) in items.iter().enumerate() {
        let next = items.get(k + 1);
        let (class, index) = if k == 0 && it.step == 0 {
            ("fragment fade-out", next.map_or(1, |n| n.step))
        } else if next.is_some() {
            ("fragment fade-in-then-out", it.step)
        } else {
            ("fragment", it.step)
        };
        out += &format!("<{tag} class=\"{class}\" data-fragment-index=\"{index}\">{}</{tag}>", render(it));
    }
    out
}

pub(super) fn notes_html(notes: &[String]) -> String {
    if notes.is_empty() {
        return String::new();
    }
    let marked = notes.iter().any(|n| n.contains("[click"));
    let mut step = 0;
    let paras: String = notes
        .iter()
        .map(|n| format!("<p>{}</p>", if marked { by_click(&inline(n), &mut step) } else { inline(n) }))
        .collect();
    format!("<aside class=\"notes\">{paras}</aside>")
}

/// Notes split by click, as in Slidev: `[click]` moves to the next click, `[click:3]` to click 3.
/// Each passage is a `<span class="note-click" data-click="n">` that the speaker view highlights
/// when the slide reaches that click. `step`: the current click, from one paragraph to the next.
fn by_click(html: &str, step: &mut u32) -> String {
    let span = |text: &str, n: u32| {
        if text.trim().is_empty() {
            String::new()
        } else {
            format!("<span class=\"note-click\" data-click=\"{n}\">{}</span>", text.trim())
        }
    };
    let mut out = String::new();
    // start of the current passage, and where to look for the next marker: a malformed marker
    // (`[click:x]`) stays in its passage as text
    let (mut passage, mut from) = (0, 0);
    while let Some(found) = html[from..].find("[click") {
        let at = from + found;
        let Some(len) = html[at..].find(']') else { break };
        let marker = &html[at + 6..at + len];
        let next = match marker.strip_prefix(':').map(str::trim) {
            Some(n) => n.parse().ok(),
            None => marker.is_empty().then_some(*step + 1),
        };
        from = at + len + 1;
        let Some(next) = next else { continue };
        out += &span(&html[passage..at], *step);
        out += " ";
        *step = next;
        passage = from;
    }
    out += &span(&html[passage..], *step);
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::step_of;

    #[test]
    fn notes_follow_the_clicks() {
        assert_eq!(notes_html(&[]), "");
        assert_eq!(notes_html(&["No mark".into()]), "<aside class=\"notes\"><p>No mark</p></aside>");
        assert_eq!(
            notes_html(&["Intro [click] then A".into(), "[click:3] finally C [clic] [click:x] D".into()]),
            "<aside class=\"notes\"><p><span class=\"note-click\" data-click=\"0\">Intro</span> \
             <span class=\"note-click\" data-click=\"1\">then A</span></p>\
             <p><span class=\"note-click\" data-click=\"3\">finally C [clic] [click:x] D</span></p></aside>"
        );
    }

    #[test]
    fn each_highlight_gets_its_clicks() {
        let steps = fx_steps("<li data-fx=\"halo:2-3 magnify:5\">a</li>");
        assert_eq!(steps.matches("fx-step").count(), 3);
        for n in [2, 4, 5] {
            assert!(steps.contains(&format!("data-fragment-index=\"{n}\"")), "{steps}");
        }
    }

    #[test]
    fn diagram_steps_include_disappearances_and_emphases() {
        let steps = skemo_steps("<g data-step=\"1\" data-until=\"2\"></g><g data-focus=\"4 6-7\"></g>");
        let indexes: Vec<&str> = steps.split("data-fragment-index=\"").skip(1).map(|s| &s[..1]).collect();
        assert_eq!(indexes, ["1", "3", "4", "5", "6", "8"]);
    }

    #[test]
    fn an_element_appears_then_leaves() {
        let a = appear(&step_of("x @2-3:up @4:halo"));
        assert_eq!((a.class.as_str(), a.attrs.as_str()), (" fragment fade-up", " data-fragment-index=\"2\""));
        assert_eq!(a.end, "<span class=\"fragment until\" data-fragment-index=\"4\"></span>");
        assert_eq!(a.fx, " data-fx=\"halo:4\"");
        assert_eq!(stepped_html("li", &step_of("y @0:dim")), "<li class=\"semi-fade-out\">y</li>");
        for (short, class) in [("down", "fade-down"), ("left", "fade-left"), ("right", "fade-right"), ("zoom", "zoom-in")] {
            assert_eq!(effect_class(short), class);
        }
        assert_eq!(effect_class("fade-in-then-out"), "fade-in-then-out");
    }

    #[test]
    fn blocks_replace_one_another() {
        let items = [step_of("a"), step_of("b @1"), step_of("c @2")];
        let html = replacing(&items, "p", |it| it.text.clone());
        assert_eq!(
            html,
            "<p class=\"fragment fade-out\" data-fragment-index=\"1\">a</p>\
             <p class=\"fragment fade-in-then-out\" data-fragment-index=\"1\">b</p>\
             <p class=\"fragment\" data-fragment-index=\"2\">c</p>"
        );
    }
}
