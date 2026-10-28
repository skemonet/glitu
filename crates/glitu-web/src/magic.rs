//! Code animé (` ````md magic-move `) : le build superpose les versions du bloc dans un même
//! `<pre class="magic-move">`, chacune après la première étant un fragment, et donne à chaque
//! mot un identifiant (`data-mm`) commun aux versions qui le gardent. Au passage d'une version
//! à l'autre, les mots gardés glissent, les nouveaux apparaissent, les autres s'effacent.

use std::collections::HashMap;

use web_sys::Element;

use crate::dom::{self, add, closest, has, qsa, remove};
use crate::engine::{self, Ev};
use crate::motion::{self, Move, Rect};

fn words(code: &Element) -> Vec<(String, Element)> {
    qsa(code, ".mm").into_iter().filter_map(|w| Some((w.get_attribute("data-mm")?, w))).collect()
}

/// De la version `from`, visible jusqu'ici, à la version `to`
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
    // L'ancienne version reste affichée le temps que ses mots perdus s'effacent
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
