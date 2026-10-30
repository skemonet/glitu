//! Small HTML tools: escaping, inline markdown, French spacing.

use pulldown_cmark::{Event, Options, Parser, html::push_html};

use crate::math;
use crate::markdown::{Stepped, marker};
use crate::render::appear;

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// French spacing: the space typed before : ; ! ? » and after « becomes non-breaking, so that
/// the sign never starts a line. English text has no such space and is left untouched.
pub fn french_spacing(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let chars: Vec<char> = html.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        let prev = i.checked_sub(1).map(|p| chars[p]);
        if c == ' ' && (matches!(next, Some(':' | ';' | '!' | '?' | '»')) || prev == Some('«')) {
            out.push('\u{a0}');
        } else {
            out.push(c);
        }
    }
    out
}

/// `$…$` and `$$…$$` formulas as MathML, everything else as is
pub fn with_math<'a>(events: impl Iterator<Item = Event<'a>>) -> impl Iterator<Item = Event<'a>> {
    events.map(|e| match e {
        Event::InlineMath(tex) => Event::InlineHtml(math::mathml(&tex, false).into()),
        Event::DisplayMath(tex) => Event::InlineHtml(math::mathml(&tex, true).into()),
        e => e,
    })
}

/// Inline markdown (`code`, *emphasis*, links, formulas, `[text]{…}`) → HTML, without an
/// enclosing paragraph
pub fn inline(md: &str) -> String {
    let mut html = String::new();
    push_html(&mut html, with_math(Parser::new_ext(&spans(md), Options::ENABLE_MATH)));
    let html = html.trim_end();
    let html = html.strip_prefix("<p>").and_then(|h| h.strip_suffix("</p>")).unwrap_or(html);
    french_spacing(html)
}

/// Hand-drawn marks around a text (`[text]{.circle}`), like Slidev's `v-mark`
const MARKS: [&str; 5] = ["underline", "circle", "box", "highlight", "strike"];

/// `[text]{.class @2}` → `<span>`: the text appears at click 2; with a mark (`.circle`,
/// `.underline`, `.box`, `.highlight`, `.strike`), the text is there from the start and the mark
/// is drawn at the click. Outside `code`, and not on a link `[x](…)`.
fn spans(md: &str) -> String {
    outside_code(md, |text| icons(&brackets(text)))
}

/// Applies `f` to the markdown outside inline `code`, which is copied as is
fn outside_code(md: &str, f: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(t) = rest.find('`') {
        let run = rest[t..].bytes().take_while(|&b| b == b'`').count();
        let close = rest[t + run..].find(&rest[t..t + run]).map_or(rest.len(), |c| t + run + c + run);
        out += &f(&rest[..t]);
        out += &rest[t..close];
        rest = &rest[close..];
    }
    out + &f(rest)
}

/// `[text]{…}` → `<span>` (see `span_html`); a link `[x](…)` stays a link
fn brackets(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(open) = rest.find('[') {
        let span = rest[open..].find(']').and_then(|c| {
            let attrs = rest[open + c + 1..].strip_prefix('{')?;
            let end = attrs.find('}')?;
            Some((&rest[open + 1..open + c], &attrs[..end], open + c + 1 + end + 2))
        });
        let Some((text, attrs, len)) = span else {
            out += &rest[..open + 1];
            rest = &rest[open + 1..];
            continue;
        };
        out += &rest[..open];
        out += &span_html(text, attrs);
        rest = &rest[len..];
    }
    out + rest
}

/// `:mdi:github:` → `<i class="icon" data-icon="mdi:github"></i>`, which the build replaces with
/// the icon (Iconify collection `mdi`, icon `github`)
fn icons(md: &str) -> String {
    let word = |w: &str| {
        !w.is_empty()
            && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && w.bytes().any(|b| b.is_ascii_lowercase())
    };
    let parts: Vec<&str> = md.split(':').collect();
    let mut out = String::with_capacity(md.len());
    let mut i = 0;
    while i < parts.len() {
        out += parts[i];
        // An icon: two words between three colons (one is needed after the second word)
        if i + 3 < parts.len() && word(parts[i + 1]) && word(parts[i + 2]) {
            out += &format!("<i class=\"icon\" data-icon=\"{}:{}\"></i>", parts[i + 1], parts[i + 2]);
            i += 3;
        } else {
            if i + 1 < parts.len() {
                out.push(':')
            }
            i += 1;
        }
    }
    out
}

fn span_html(text: &str, attrs: &str) -> String {
    let mut classes: Vec<&str> = Vec::new();
    let mut step = Stepped::default();
    let mut id = String::new();
    for tok in attrs.split_whitespace() {
        if let Some(c) = tok.strip_prefix('.') {
            classes.push(c);
        } else if let Some(name) = tok.strip_prefix('#') {
            // counterpart from one animated slide to the next
            id = format!(" data-id=\"{}\"", esc(name));
        } else if let Some(m) = marker(tok) {
            step.add(m);
        }
    }
    let a = appear(&step);
    let Some(mark) = classes.iter().copied().find(|c| MARKS.contains(c)) else {
        let class = format!("{}{}", classes.join(" "), a.class);
        let class = if class.trim().is_empty() { String::new() } else { format!(" class=\"{}\"", class.trim()) };
        return format!("<span{class}{id}{}{}>{text}{}</span>", a.attrs, a.fx, a.end);
    };
    // The mark is drawn when its fragment appears (at the click), or right away without a click
    let trigger =
        if step.step > 0 { format!("<span class=\"fragment mark-on\"{}></span>", a.attrs) } else { String::new() };
    let others: String = classes.iter().filter(|c| **c != mark).map(|c| format!(" {c}")).collect();
    format!("<span class=\"mark mark-{mark}{others}\"{id}{}>{trigger}{text}{}</span>", a.fx, mark_svg(mark, text))
}

/// A small deterministic random generator seeded by a text, so that the same text always gets
/// the same hand-drawn stroke
struct Jitter(u32);

impl Jitter {
    fn of(text: &str) -> Jitter {
        Jitter(text.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619)))
    }

    /// `v` moved by at most `amp` either way, written with one decimal
    fn around(&mut self, v: f64, amp: f64) -> String {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        format!("{:.1}", v + (self.0 % 1000) as f64 / 1000.0 * 2.0 * amp - amp)
    }
}

/// The « hand-drawn » stroke of a mark: an SVG stretched over the text (viewBox 100 × 100, stroke
/// not distorted), its irregularities drawn from the text so that the same text keeps the same stroke
fn mark_svg(kind: &str, seed_text: &str) -> String {
    let mut r = Jitter::of(seed_text);
    let mut j = |v: f64, amp: f64| r.around(v, amp);
    let d = match kind {
        "underline" => format!(
            "M{} {} Q50 {} {} {} M{} {} Q55 {} {} {}",
            j(0.0, 2.0),
            j(96.0, 2.0),
            j(92.0, 3.0),
            j(100.0, 2.0),
            j(94.0, 2.0),
            j(4.0, 2.0),
            j(99.0, 1.0),
            j(97.0, 2.0),
            j(97.0, 2.0),
            j(100.0, 1.0)
        ),
        "strike" => format!("M{} {} L{} {}", j(-2.0, 1.0), j(55.0, 4.0), j(102.0, 1.0), j(48.0, 4.0)),
        // exactly over the marked text: the spaces around it stay as they are
        "highlight" => format!("M{} {} L{} {}", j(0.5, 0.5), j(58.0, 3.0), j(99.5, 0.5), j(56.0, 3.0)),
        "box" => format!(
            "M{} {} L{} {} L{} {} L{} {} Z M{} {} L{} {}",
            j(-3.0, 1.5),
            j(-6.0, 3.0),
            j(103.0, 1.5),
            j(-4.0, 3.0),
            j(102.0, 1.5),
            j(106.0, 3.0),
            j(-2.0, 1.5),
            j(104.0, 3.0),
            j(-4.0, 1.5),
            j(-2.0, 3.0),
            j(20.0, 3.0),
            j(-7.0, 3.0)
        ),
        // An ellipse a little more than one turn, which does not quite close
        _ => format!(
            "M{} {} C{} {} {} {} {} {} C{} {} {} {} {} {} C{} {} {} {} {} {}",
            j(8.0, 3.0),
            j(6.0, 3.0),
            j(55.0, 4.0),
            j(-6.0, 3.0),
            j(106.0, 2.0),
            j(8.0, 3.0),
            j(103.0, 2.0),
            j(55.0, 3.0),
            j(100.0, 2.0),
            j(108.0, 2.0),
            j(12.0, 3.0),
            j(110.0, 2.0),
            j(-3.0, 2.0),
            j(56.0, 3.0),
            j(-6.0, 2.0),
            j(18.0, 3.0),
            j(30.0, 3.0),
            j(-5.0, 2.0),
            j(72.0, 3.0),
            j(-1.0, 2.0)
        ),
    };
    format!(
        "<svg class=\"mark-svg\" viewBox=\"0 0 100 100\" preserveAspectRatio=\"none\" aria-hidden=\"true\">\
         <path d=\"{d}\" vector-effect=\"non-scaling-stroke\"/></svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_and_spacing() {
        assert_eq!(inline("Se connecter : `use-context`"), "Se connecter\u{a0}: <code>use-context</code>");
        assert_eq!(inline("une IP « interne » kube"), "une IP «\u{a0}interne\u{a0}» kube");
        assert_eq!(inline("No space: unchanged"), "No space: unchanged");
        assert_eq!(esc("<a href=\"x\">"), "&lt;a href=&quot;x&quot;&gt;");
        assert!(inline("area $\\pi r^2$").contains("<math"));
        assert!(inline("$$x$$").contains("display=\"block\""));
        assert_eq!(
            inline("see :mdi:github: or `:a:b:`"),
            "see <i class=\"icon\" data-icon=\"mdi:github\"></i> or <code>:a:b:</code>"
        );
        assert_eq!(inline("at 10:30:45 : end"), "at 10:30:45\u{a0}: end");
        assert_eq!(inline("a [word]{@2} here"), "a <span class=\"fragment\" data-fragment-index=\"2\">word</span> here");
        assert_eq!(inline("`[a]{@1}` and [link](x)"), "<code>[a]{@1}</code> and <a href=\"x\">link</a>");
        assert_eq!(inline("[a]{#k .big}"), "<span class=\"big\" data-id=\"k\">a</span>");
        assert_eq!(inline("[plain]{}"), "<span>plain</span>");
        assert_eq!(inline("unclosed [x] and [y]{"), "unclosed [x] and [y]{");
        assert!(!inline("the price is $5 and $10").contains("<math"));
    }

    #[test]
    fn marks_are_drawn_at_their_click() {
        let m = inline("[key]{.circle .red @3}");
        assert!(m.starts_with(
            "<span class=\"mark mark-circle red\"><span class=\"fragment mark-on\" data-fragment-index=\"3\"></span>key<svg"
        ));
        for kind in MARKS {
            let m = inline(&format!("[word]{{.{kind}}}"));
            assert!(m.starts_with(&format!("<span class=\"mark mark-{kind}\">word<svg")), "{m}");
        }
        // the same text keeps the same stroke
        assert_eq!(mark_svg("box", "a"), mark_svg("box", "a"));
        assert_ne!(mark_svg("box", "a"), mark_svg("box", "b"));
    }
}
