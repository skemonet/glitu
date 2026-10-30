//! Animated code (` ````md magic-move `): the build stacks the versions of the block in one
//! `<pre class="magic-move">`, each one after the first being a fragment, and gives each word an
//! identifier (`data-mm`) shared by the versions that keep it. Going from one version to the
//! next, the words kept slide, the new ones appear, the others fade out.

use std::collections::HashMap;

use web_sys::Element;

use crate::dom::{self, add, closest, has, qsa, remove};
use crate::engine::{self, Ev};
use crate::motion::{self, Move, Rect};

fn words(code: &Element) -> Vec<(String, Element)> {
    qsa(code, ".mm").into_iter().filter_map(|w| Some((w.get_attribute("data-mm")?, w))).collect()
}

/// From version `from`, visible until now, to version `to`
fn transition(from: &Element, to: &Element) {
    if motion::reduced() {
        return;
    }
    let before: HashMap<String, Element> = words(from).into_iter().collect();
    let moves = words(to)
        .into_iter()
        .map(|(id, w)| match before.get(&id) {
            Some(old) => {
                add(old, "mm-kept");
                (w, Move::From(Rect::of(old)))
            }
            None => (w, Move::Appear),
        })
        .collect();
    // The old version stays shown while its lost words fade out
    add(from, "mm-leaving");
    motion::play(moves);
    let from = from.clone();
    dom::timeout(motion::DURATION, move || {
        remove(&from, "mm-leaving");
        qsa(&from, ".mm-kept").iter().for_each(|w| remove(w, "mm-kept"));
    });
}

fn versions(frags: &[Element]) -> impl Iterator<Item = (&Element, Element)> {
    frags.iter().filter(|f| closest(f, "pre.magic-move").is_some() && has(f, "fragment")).filter_map(|f| Some((f, f.previous_element_sibling()?)))
}

pub fn install() {
    engine::on(|ev| match ev {
        Ev::FragmentShown(frags) => versions(frags).for_each(|(to, from)| transition(&from, to)),
        Ev::FragmentHidden(frags) => versions(frags).for_each(|(from, to)| transition(from, &to)),
        _ => {}
    });
}
