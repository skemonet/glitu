//! Mouvements « FLIP » communs au code animé (`magic`) et aux slides animées (`animate`) :
//! un élément déjà à sa nouvelle place est d'abord ramené, par une transformation, là où se
//! trouvait son homologue, puis rendu à sa place par une transition. Les éléments sans
//! homologue apparaissent en fondu.

use web_sys::Element;

use crate::dom::{self, style, unstyle};

/// Durée d'un mouvement (ms)
pub const DURATION: f64 = 600.0;
const EASE: &str = "cubic-bezier(.4, 0, .2, 1)";

/// Une boîte, en coordonnées de la fenêtre
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn of(el: &Element) -> Rect {
        let r = el.get_bounding_client_rect();
        Rect { x: r.left(), y: r.top(), w: r.width(), h: r.height() }
    }

    /// La même boîte, relative à `from` puis reportée sur `to`
    pub fn moved(self, from: Rect, to: Rect) -> Rect {
        Rect { x: self.x - from.x + to.x, y: self.y - from.y + to.y, ..self }
    }
}

pub enum Move {
    /// D'où part l'élément : la boîte de son homologue
    From(Rect),
    /// Pas d'homologue : fondu, après que les autres ont commencé à bouger
    Appear,
}

/// L'utilisateur (ou l'export) ne veut pas d'animations
pub fn reduced() -> bool {
    dom::window().match_media("(prefers-reduced-motion: reduce)").ok().flatten().is_some_and(|m| m.matches())
}

/// Joue les mouvements : toutes les positions de départ d'abord, une seule mise en page,
/// puis toutes les transitions
pub fn play(moves: Vec<(Element, Move)>) {
    if moves.is_empty() || reduced() {
        return;
    }
    let mut started = Vec::with_capacity(moves.len());
    for (el, m) in moves {
        style(&el, "transition", "none");
        match m {
            Move::From(old) => {
                let new = Rect::of(&el);
                if new.w == 0.0 || new.h == 0.0 || old.w == 0.0 || old.h == 0.0 {
                    unstyle(&el, "transition");
                    continue;
                }
                // Coordonnées de la fenêtre → celles de l'élément, que les slides mettent à l'échelle
                let scale = new.w / f64::from(dom::html(&el).offset_width().max(1));
                let (dx, dy) = ((old.x - new.x) / scale, (old.y - new.y) / scale);
                let (sx, sy) = (old.w / new.w, old.h / new.h);
                if dx.abs() < 0.5 && dy.abs() < 0.5 && (sx - 1.0).abs() < 0.01 && (sy - 1.0).abs() < 0.01 {
                    unstyle(&el, "transition");
                    continue;
                }
                style(&el, "transform-origin", "0 0");
                style(&el, "transform", &format!("translate({dx}px, {dy}px) scale({sx}, {sy})"));
                started.push((el, format!("transform {DURATION}ms {EASE}")));
            }
            Move::Appear => {
                style(&el, "opacity", "0");
                started.push((el, format!("opacity {}ms ease {}ms", DURATION / 2.0, DURATION / 3.0)));
            }
        }
    }
    // Une lecture force la mise en page : les positions de départ sont prises en compte
    let _ = dom::document().body().map(|b| b.offset_height());
    for (el, transition) in &started {
        style(el, "transition", transition);
        unstyle(el, "transform");
        unstyle(el, "opacity");
    }
    dom::timeout(DURATION * 1.5, move || {
        for (el, _) in started {
            for p in ["transition", "transform-origin"] {
                unstyle(&el, p);
            }
        }
    });
}
