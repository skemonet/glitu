//! Attributes written in comments in the markdown, rule of reveal.js's markdown plugin:
//!
//! ```text
//!   <!-- .element: class="fragment" data-fragment-index="2" -->   on the previous sibling
//!                                                                 element, otherwise the parent
//!   <!-- .slide: data-background="#000" -->                       on the slide
//! ```
//!
//! The comments are read in the DOM of each slide, before everything else.

use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

const ELEMENT: u16 = 1;
const COMMENT: u16 = 8;

/// `key="value"` or `data-x` alone
fn pairs(spec: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = spec;
    while !rest.is_empty() {
        rest = rest.trim_start();
        let end = rest.find(['=', ' ', '"']).unwrap_or(rest.len());
        let key = &rest[..end];
        let after = &rest[end..];
        if let Some(v) = after.strip_prefix("=\"") {
            let close = v.find('"').unwrap_or(v.len());
            if !key.is_empty() && close > 0 {
                out.push((key.to_string(), v[..close].to_string()));
            }
            rest = v.get(close + 1..).unwrap_or("");
        } else {
            if key.starts_with("data-") && (after.starts_with(' ') || after.starts_with('"')) {
                out.push((key.to_string(), String::new()));
            }
            rest = after.get(1..).unwrap_or("");
        }
    }
    out
}

/// Removes the marker from the comment and sets its attributes on `target`; false if it is absent
fn apply_marker(comment: &Node, target: &Element, marker: &str) -> bool {
    let value = comment.node_value().unwrap_or_default();
    let Some(at) = value.find(marker) else { return false };
    let after = &value[at + marker.len()..];
    let line_end = after.find('\n').unwrap_or(after.len());
    let spec = after[..line_end].trim_start_matches(':').trim();
    if spec.is_empty() {
        return false;
    }
    comment.set_node_value(Some(&format!("{}{}", &value[..at], &after[line_end..])));
    for (k, v) in pairs(spec) {
        let _ = target.set_attribute(&k, &v);
    }
    true
}

fn visit(section: &Element, node: &Node, previous: &Element) {
    let children = node.child_nodes();
    if children.length() > 0 {
        let mut previous_parent: Element = node.clone().unchecked_into();
        for i in 0..children.length() {
            let child = children.item(i).unwrap();
            for j in (0..i).rev() {
                let c = children.item(j).unwrap();
                if c.node_type() == ELEMENT && c.node_name() != "BR" {
                    previous_parent = c.unchecked_into();
                    break;
                }
            }
            if matches!(child.node_type(), ELEMENT | COMMENT) {
                visit(section, &child, &previous_parent);
            }
        }
    }
    if node.node_type() == COMMENT && !apply_marker(node, previous, ".element") {
        apply_marker(node, section, ".slide");
    }
}

pub fn apply(section: &Element) {
    visit(section, section, section);
}
