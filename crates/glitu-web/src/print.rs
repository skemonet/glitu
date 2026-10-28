//! Export PDF (`?print-pdf` dans l'adresse, puis Imprimer → Enregistrer en PDF), comme
//! reveal.js : une page par slide (`.pdf-page`, que reveal.css habille), toutes les étapes
//! affichées, au format de la slide.
//!
//! La vue défilante (`?view=scroll`, ou d'office sur un téléphone) reprend ces pages, mises à
//! la largeur de la fenêtre : la présentation se lit comme une page web.

use crate::dom::{self, add, style};
use crate::engine;

/// Met les slides en pages ; appelé avant `engine::ready`, pour que les composants trouvent
/// leurs slides déjà en place
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

/// Vue défilante : les pages à la largeur de la fenêtre
fn fit_width(html: &web_sys::Element, page_width: f64) {
    // sans la barre de défilement
    let width = f64::from(html.client_width());
    let width = if width > 0.0 { width } else { page_width };
    style(html, "--page-zoom", &format!("{}", width / page_width));
}
