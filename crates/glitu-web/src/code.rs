//! Code surligné par étapes : à chaque étape, le bloc défile pour centrer les lignes
//! surlignées (comme le plugin highlight de reveal.js). Le build produit une copie du bloc
//! par étape (`code.fragment`), toutes superposées dans le même `<pre>`.

use std::cell::RefCell;
use std::rc::Rc;

use web_sys::Element;

use crate::dom::{self, has, qsa};
use crate::engine::{self, Ev};

#[derive(Default)]
struct ScrollState {
    current: Option<Element>,
    raf: i32,
}

thread_local! {
    /// Un état de défilement par `<pre>`
    static STATES: RefCell<Vec<(Element, Rc<RefCell<ScrollState>>)>> = const { RefCell::new(Vec::new()) };
}

fn state_of(pre: &Element) -> Rc<RefCell<ScrollState>> {
    STATES.with(|s| {
        let mut s = s.borrow_mut();
        if let Some((_, st)) = s.iter().find(|(p, _)| p == pre) {
            return Rc::clone(st);
        }
        let st = Rc::new(RefCell::new(ScrollState::default()));
        s.push((pre.clone(), Rc::clone(&st)));
        st
    })
}

fn ease(t: f64) -> f64 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        let t = t - 1.0;
        1.0 - 8.0 * t * t * t * t
    }
}

fn px(v: &str) -> f64 {
    v.trim_end_matches("px").parse::<f64>().map_or(0.0, f64::trunc)
}

fn scroll_into_view(block: &Element, skip_animation: bool) {
    let Some(pre) = block.parent_element() else { return };
    let st = state_of(&pre);
    dom::cancel_raf(st.borrow().raf);
    let b = dom::html(block);
    if let Some(cur) = st.borrow().current.as_ref() {
        b.set_scroll_top(dom::html(cur).scroll_top());
    }
    st.borrow_mut().current = Some(block.clone());

    let lines = qsa(block, ".highlight-line");
    let span = || match (lines.first(), lines.last()) {
        (Some(f), Some(l)) => {
            let (f, l) = (dom::html(f), dom::html(l));
            (f.offset_top() as f64, (l.offset_top() + l.offset_height()) as f64)
        }
        _ => (0.0, 0.0),
    };
    let css = dom::window().get_computed_style(block).ok().flatten();
    let pad = |p: &str| css.as_ref().and_then(|c| c.get_property_value(p).ok()).map_or(0.0, |v| px(&v));
    let (pt, pb) = (pad("padding-top"), pad("padding-bottom"));
    let viewport = b.offset_height() as f64 - pt - pb;
    let (top, bottom) = span();
    let start = b.scroll_top() as f64;
    let mut target = top + ((bottom - top).min(viewport) - viewport) / 2.0;
    if let Some(table) = dom::qs(block, ".hljs-ln") {
        target += dom::html(&table).offset_top() as f64 - pt;
    }
    let target = target.min(b.scroll_height() as f64 - viewport).max(0.0);

    if skip_animation || start == target {
        b.set_scroll_top(target as i32);
        return;
    }
    if (b.scroll_height() as f64) <= viewport {
        return;
    }
    animate(block.clone(), st, start, target, 0.0);
}

fn animate(block: Element, st: Rc<RefCell<ScrollState>>, start: f64, target: f64, time: f64) {
    let time = (time + 0.02).min(1.0);
    dom::html(&block).set_scroll_top((start + (target - start) * ease(time)) as i32);
    if time < 1.0 {
        let st2 = Rc::clone(&st);
        let id = dom::raf(move |_| animate(block, st2, start, target, time));
        st.borrow_mut().raf = id;
    }
}

/// À la première arrivée sur la slide, le premier surlignage est centré sans animation
fn first_blocks(slide: &Element) {
    for pre in qsa(slide, "pre.code-wrapper") {
        let Some(first) = dom::qs(&pre, "code.has-highlights") else { continue };
        if first.has_attribute("data-glitu-scrolled") {
            continue;
        }
        let _ = first.set_attribute("data-glitu-scrolled", "");
        scroll_into_view(&first, true);
    }
}

pub fn install() {
    engine::on(|ev| match ev {
        Ev::Ready | Ev::SlideChanged { .. } => first_blocks(&engine::current_slide()),
        Ev::FragmentShown(frags) => {
            for f in frags.iter().filter(|f| has(f, "has-highlights")) {
                scroll_into_view(f, false);
            }
        }
        Ev::FragmentHidden(frags) => {
            for f in frags.iter().filter(|f| has(f, "has-highlights")) {
                if let Some(prev) = f.previous_element_sibling() {
                    scroll_into_view(&prev, false);
                }
            }
        }
        Ev::Resize => {}
    });
}
