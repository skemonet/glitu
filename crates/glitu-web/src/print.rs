//! PDF export (`?print-pdf` in the address, then Print → Save as PDF), as in reveal.js: one page
//! per slide (`.pdf-page`, which reveal.css dresses), all the steps shown, in the slide's format.
//!
//! The scroll view (`?view=scroll`, or by default on a phone) takes these pages, at the width of
//! the window: the presentation reads like a web page.

use crate::dom::{self, add, style};
use crate::engine;

/// Lays the slides out as pages; called before `engine::ready`, so that the components find
/// their slides already in place
pub fn install() {
    if !engine::printing() {
        return;
    }
    let c = engine::config();
    let (sw, sh) = (c.width, c.height);
    let (pw, ph) = ((sw * (1.0 + c.margin)).floor(), (sh * (1.0 + c.margin)).floor());
    let scrolling = engine::scrolling();
    let page_rule = if scrolling { String::new() } else { format!("@page {{ size: {pw}px {ph}px; margin: 0px; }}") };
    let sheet = dom::create("style");
    sheet.set_text_content(Some(&format!(
        "{page_rule}\n\
         .reveal section>img, .reveal section>video, .reveal section>iframe {{ max-width: {sw}px; max-height: {sh}px; }}\n\
         .print-pdf .reveal .slides > section.stack {{ display: none !important; }}"
    )));
    let _ = engine::root().append_child(&sheet);
    let html = dom::document().document_element().unwrap();
    add(&html, "reveal-print");
    add(&html, "print-pdf");
    let body: web_sys::Element = dom::document().body().unwrap().into();
    if scrolling {
        add(&html, "scroll-view");
        fit_width(&html, pw);
        let html2 = html.clone();
        dom::on::<web_sys::Event>(&dom::window(), "resize", move |_| fit_width(&html2, pw));
    } else {
        style(&body, "width", &format!("{pw}px"));
        style(&body, "height", &format!("{ph}px"));
    }

    let container = engine::slides_element();
    for (_, _, slide) in engine::all_leaves() {
        let page = dom::create("div");
        page.set_class_name("pdf-page");
        style(&page, "height", &format!("{ph}px"));
        let _ = page.append_child(&slide);
        style(&slide, "left", &format!("{}px", (pw - sw) / 2.0));
        style(&slide, "top", &format!("{}px", (ph - sh) / 2.0));
        style(&slide, "width", &format!("{sw}px"));
        style(&slide, "height", &format!("{sh}px"));
        let _ = container.append_child(&page);
    }
}

/// Scroll view: the pages at the width of the window
fn fit_width(html: &web_sys::Element, page_width: f64) {
    // without the scroll bar
    let width = f64::from(html.client_width());
    let width = if width > 0.0 { width } else { page_width };
    style(html, "--page-zoom", &format!("{}", width / page_width));
}
