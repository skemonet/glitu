//! Media, as in reveal.js: `data-src` loads when the slide gets close (images, videos, sounds,
//! iframes); `data-autoplay` starts the video or the sound on arriving on the slide; on leaving
//! it, what plays stops (except `data-ignore`).

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlMediaElement};

use crate::dom::{data, qsa};
use crate::engine::{self, Ev};

const LAZY: &str = "img[data-src], video[data-src], audio[data-src], iframe[data-src], source[data-src]";

/// Loads the `data-src` of the slides close to the current one
fn load_near() {
    let (h, v, _) = engine::position();
    for (x, y, slide) in engine::all_leaves() {
        if engine::distance(x, y, h, v) >= engine::VIEW_DISTANCE {
            continue;
        }
        for el in qsa(&slide, LAZY) {
            if let Some(src) = data(&el, "src") {
                let _ = el.set_attribute("src", &src);
                let _ = el.remove_attribute("data-src");
                // A <source> only loads by restarting its player
                if el.tag_name() == "SOURCE"
                    && let Some(m) = el.parent_element().and_then(|p| p.dyn_into::<HtmlMediaElement>().ok())
                {
                    m.load();
                }
            }
        }
    }
}

fn media(slide: &Element, selector: &str) -> Vec<HtmlMediaElement> {
    qsa(slide, selector).into_iter().filter_map(|e| e.dyn_into().ok()).collect()
}

fn stop(slide: &Element) {
    for m in media(slide, "video:not([data-ignore]), audio:not([data-ignore])") {
        let _ = m.pause();
    }
}

fn autoplay(slide: &Element) {
    for m in media(slide, "video[data-autoplay], audio[data-autoplay]") {
        m.set_current_time(0.0);
        // The browser may refuse to play without a gesture of the user: do not insist
        let _ = m.play();
    }
}

pub fn install() {
    engine::on(|ev| match ev {
        Ev::Ready => {
            load_near();
            autoplay(&engine::current_slide());
        }
        Ev::SlideChanged { previous } => {
            if let Some(p) = previous {
                stop(p);
            }
            load_near();
            autoplay(&engine::current_slide());
        }
        _ => {}
    });
}
