//! The objects of a slide: each element surveyed by `assets/export.js` (shape, path, text, image,
//! group) becomes an independent DrawingML object.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

use super::esc;

/// What writing a slide accumulates: object numbers, images and links
pub(super) struct Sheet {
    /// EMU per page pixel
    emu: f64,
    /// Office language tag of the texts (`en-US`)
    lang: String,
    next_id: u32,
    rels: Vec<(&'static str, String)>,
    /// The slide's images: (relationship number, content, extension)
    pub(super) media: Vec<(usize, Vec<u8>, &'static str)>,
}

impl Sheet {
    /// A slide `width` pixels wide over `slide_emu` EMU, its texts in `lang`
    pub(super) fn new(width: u32, slide_emu: u64, lang: &str) -> Sheet {
        Sheet { emu: slide_emu as f64 / f64::from(width), lang: lang.into(), next_id: 1, rels: Vec::new(), media: Vec::new() }
    }

    /// Its relationships (rId1, rId2…)
    pub(super) fn rels(&self) -> &[(&'static str, String)] {
        &self.rels
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
    fn e(&self, px: f64) -> i64 {
        (px * self.emu).round() as i64
    }
    /// Points (hundredths): font sizes, line spacing, letter spacing
    fn pt100(&self, px: f64) -> i64 {
        (px * self.emu / 12_700.0 * 100.0).round() as i64
    }
    pub(super) fn rel(&mut self, kind: &'static str, target: String) -> String {
        self.rels.push((kind, target));
        format!("rId{}", self.rels.len())
    }
    pub(super) fn image(&mut self, bytes: Vec<u8>, ext: &'static str) -> String {
        let n = self.media.len();
        let id = self.rel("image", format!("../media/@{n}.{ext}"));
        self.media.push((self.rels.len(), bytes, ext));
        id
    }
}

fn num(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or(0.0)
}

/// `[r, g, b, a]` → `<a:srgbClr>`
fn rgba(c: &Value) -> Option<String> {
    let a = c.as_array()?;
    let ch = |i: usize| a.get(i).and_then(Value::as_f64).unwrap_or(0.0).round().clamp(0.0, 255.0) as u8;
    let alpha = a.get(3).and_then(Value::as_f64).unwrap_or(1.0).clamp(0.0, 1.0);
    let hex = format!("{:02X}{:02X}{:02X}", ch(0), ch(1), ch(2));
    Some(if alpha >= 0.999 {
        format!("<a:srgbClr val=\"{hex}\"/>")
    } else {
        format!("<a:srgbClr val=\"{hex}\"><a:alpha val=\"{}\"/></a:srgbClr>", (alpha * 100_000.0).round() as i64)
    })
}

fn solid(c: &Value) -> String {
    rgba(c).map_or_else(|| "<a:noFill/>".into(), |c| format!("<a:solidFill>{c}</a:solidFill>"))
}

/// `{c}` or `{grad: {angle, radial, stops}}` → DrawingML fill
fn fill(f: &Value) -> String {
    if f.is_null() {
        return "<a:noFill/>".into();
    }
    if !f["c"].is_null() {
        return solid(&f["c"]);
    }
    let g = &f["grad"];
    let stops: String = g["stops"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| Some(format!("<a:gs pos=\"{}\">{}</a:gs>", (s[0].as_f64()?.clamp(0.0, 1.0) * 100_000.0).round() as i64, rgba(&s[1])?)))
        .collect();
    if stops.is_empty() {
        return "<a:noFill/>".into();
    }
    // CSS: 0deg upwards, 90deg to the right; DrawingML: 0 from left to right
    let shade = if g["radial"].as_bool() == Some(true) {
        "<a:path path=\"circle\"><a:fillToRect l=\"50000\" t=\"50000\" r=\"50000\" b=\"50000\"/></a:path>".to_string()
    } else {
        let ang = (num(g, "angle") - 90.0).rem_euclid(360.0);
        format!("<a:lin ang=\"{}\" scaled=\"0\"/>", (ang * 60_000.0).round() as i64)
    };
    format!("<a:gradFill rotWithShape=\"1\"><a:gsLst>{stops}</a:gsLst>{shade}</a:gradFill>")
}

fn line(sh: &Sheet, l: &Value, head: bool, tail: bool) -> String {
    if l.is_null() {
        return "<a:ln><a:noFill/></a:ln>".into();
    }
    let w = num(l, "w");
    let cap = match l["cap"].as_str() { Some("round") => " cap=\"rnd\"", Some("square") => " cap=\"sq\"", _ => "" };
    let dash = if l["dash"].as_bool() == Some(true) { "<a:prstDash val=\"dash\"/>" } else { "" };
    // Arrow: an SVG marker at the start or the end of the path
    let arrow = |tag: &str| format!("<a:{tag} type=\"triangle\" w=\"med\" len=\"med\"/>");
    format!(
        "<a:ln w=\"{}\"{cap}>{}{dash}<a:round/>{}{}</a:ln>",
        sh.e(w),
        solid(&l["c"]),
        if head { arrow("headEnd") } else { String::new() },
        if tail { arrow("tailEnd") } else { String::new() }
    )
}

fn effects(sh: &Sheet, s: &Value) -> String {
    let Some(c) = rgba(&s["c"]) else { return String::new() };
    let (dx, dy) = (num(s, "dx"), num(s, "dy"));
    let dir = (dy.atan2(dx).to_degrees().rem_euclid(360.0) * 60_000.0).round() as i64;
    format!(
        "<a:effectLst><a:outerShdw blurRad=\"{}\" dist=\"{}\" dir=\"{dir}\" algn=\"ctr\" rotWithShape=\"0\">{c}</a:outerShdw></a:effectLst>",
        sh.e(num(s, "blur")),
        sh.e((dx * dx + dy * dy).sqrt())
    )
}

fn xfrm(sh: &Sheet, x: f64, y: f64, w: f64, h: f64) -> String {
    format!("<a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>", sh.e(x), sh.e(y), sh.e(w).max(1), sh.e(h).max(1))
}

fn nv(id: u32, name: &str, extra: &str) -> String {
    format!("<p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{} {id}\"/><p:cNvSpPr{extra}/><p:nvPr/></p:nvSpPr>", esc(name))
}

/// A `<p:sp>` shape: name, frame (x, y, width, height in pixels), geometry, paint (fill, stroke,
/// effects) and, for a text box, its text
struct Sp<'a> {
    name: &'a str,
    frame: (f64, f64, f64, f64),
    geometry: String,
    paint: String,
    text: Option<String>,
}

fn sp(sh: &mut Sheet, s: Sp) -> String {
    let id = sh.id();
    let (x, y, w, h) = s.frame;
    let text_box = if s.text.is_some() { " txBox=\"1\"" } else { "" };
    format!(
        "<p:sp>{}<p:spPr>{}{}{}</p:spPr>{}</p:sp>",
        nv(id, s.name, text_box),
        xfrm(sh, x, y, w, h),
        s.geometry,
        s.paint,
        s.text.unwrap_or_default()
    )
}

/// Fill, stroke (arrows included) and shadow of a surveyed object
fn paint(sh: &Sheet, it: &Value) -> String {
    let flag = |k: &str| it[k].as_bool() == Some(true);
    format!("{}{}{}", fill(&it["fill"]), line(sh, &it["line"], flag("head"), flag("tail")), effects(sh, &it["shadow"]))
}

/// Preset geometry, with its adjustments (`adj`…) in 100,000ths
fn preset(name: &str, adjust: &[(&str, i64)]) -> String {
    let gd: String = adjust.iter().map(|(n, v)| format!("<a:gd name=\"{n}\" fmla=\"val {v}\"/>")).collect();
    format!("<a:prstGeom prst=\"{name}\"><a:avLst>{gd}</a:avLst></a:prstGeom>")
}

/// The objects of a slide, in painting order
pub(super) fn shapes(sh: &mut Sheet, items: Vec<Value>) -> String {
    merge(items).iter().map(|it| shape(sh, it)).collect()
}

fn shape(sh: &mut Sheet, it: &Value) -> String {
    match it["t"].as_str().unwrap_or("") {
        "box" => rect(sh, it),
        "path" => path(sh, it),
        "text" => text(sh, it),
        "pic" => picture(sh, it),
        "group" => group(sh, it),
        _ => String::new(),
    }
}

fn rect(sh: &mut Sheet, it: &Value) -> String {
    let (mut x, mut y, mut w, mut h) = (num(it, "x"), num(it, "y"), num(it, "w"), num(it, "h"));
    if w <= 0.0 || h <= 0.0 {
        return String::new();
    }
    // A CSS border is inside, a DrawingML stroke is centred on the outline
    let lw = if it["line"].is_null() { 0.0 } else { num(&it["line"], "w") };
    if lw > 0.0 && lw < w && lw < h {
        (x, y, w, h) = (x + lw / 2.0, y + lw / 2.0, w - lw, h - lw);
    }
    let r: Vec<f64> = it["r"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect()).unwrap_or_default();
    let r = |i: usize| r.get(i).copied().unwrap_or(0.0);
    let min = w.min(h);
    let adj = |v: f64| ((v / min * 100_000.0).round() as i64).clamp(0, 50_000);
    // Corners: none, the top two (a window's title bar), or all of them, at the largest
    let geometry = if (0..4).all(|i| r(i) < 0.5) {
        preset("rect", &[])
    } else if r(0) > 0.5 && (r(0) - r(1)).abs() < 0.5 && r(2) < 0.5 && r(3) < 0.5 {
        preset("round2SameRect", &[("adj1", adj(r(0))), ("adj2", 0)])
    } else {
        preset("roundRect", &[("adj", adj((0..4).map(r).fold(0.0, f64::max)))])
    };
    let paint = paint(sh, it);
    sp(sh, Sp { name: "Shape", frame: (x, y, w, h), geometry, paint, text: None })
}

fn path(sh: &mut Sheet, it: &Value) -> String {
    let cmds: Vec<&Vec<Value>> = it["d"].as_array().into_iter().flatten().filter_map(Value::as_array).collect();
    let pts = cmds.iter().flat_map(|c| c[1..].chunks(2).filter_map(|p| Some((p.first()?.as_f64()?, p.get(1)?.as_f64()?))));
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in pts {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    if x0 > x1 {
        return String::new();
    }
    let (w, h) = ((x1 - x0).max(0.01), (y1 - y0).max(0.01));
    let pt = |p: &[Value]| format!("<a:pt x=\"{}\" y=\"{}\"/>", sh.e(p[0].as_f64().unwrap_or(0.0) - x0), sh.e(p[1].as_f64().unwrap_or(0.0) - y0));
    let mut d = String::new();
    for c in &cmds {
        match c[0].as_str().unwrap_or("") {
            "M" if c.len() >= 3 => d += &format!("<a:moveTo>{}</a:moveTo>", pt(&c[1..3])),
            "L" if c.len() >= 3 => d += &format!("<a:lnTo>{}</a:lnTo>", pt(&c[1..3])),
            "C" if c.len() >= 7 => d += &format!("<a:cubicBezTo>{}{}{}</a:cubicBezTo>", pt(&c[1..3]), pt(&c[3..5]), pt(&c[5..7])),
            "Z" => d += "<a:close/>",
            _ => {}
        }
    }
    let open = if it["fill"].is_null() { " fill=\"none\"" } else { "" };
    let geometry = format!(
        "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/>\
<a:pathLst><a:path w=\"{}\" h=\"{}\"{open}>{d}</a:path></a:pathLst></a:custGeom>",
        sh.e(w).max(1),
        sh.e(h).max(1)
    );
    let paint = paint(sh, it);
    sp(sh, Sp { name: "Path", frame: (x0, y0, w, h), geometry, paint, text: None })
}

fn run(sh: &mut Sheet, r: &Value) -> String {
    if r["br"].as_bool() == Some(true) {
        return format!("<a:br><a:rPr lang=\"{}\" sz=\"{}\" dirty=\"0\"/></a:br>", sh.lang, sh.pt100(num(r, "size")).clamp(100, 400_000));
    }
    let flag = |k: &str| r[k].as_bool() == Some(true);
    let mut attrs = format!(" lang=\"{}\" sz=\"{}\"", sh.lang, sh.pt100(num(r, "size")).clamp(100, 400_000));
    if flag("b") { attrs += " b=\"1\"" }
    if flag("i") { attrs += " i=\"1\"" }
    if flag("u") { attrs += " u=\"sng\"" }
    if flag("s") { attrs += " strike=\"sngStrike\"" }
    if let Some(cap) = r["cap"].as_str() { attrs += &format!(" cap=\"{cap}\"") }
    let spc = sh.pt100(num(r, "spc"));
    if spc != 0 { attrs += &format!(" spc=\"{spc}\"") }
    // Gradient text (`background-clip: text`): PowerPoint knows it too
    let mut inner = String::new();
    if r["stroke"].is_object() {
        inner += &format!("<a:ln w=\"{}\">{}</a:ln>", sh.e(num(&r["stroke"], "w")), solid(&r["stroke"]["c"]));
    }
    inner += &if r["grad"].is_object() { fill(&serde_json::json!({ "grad": r["grad"] })) } else { solid(&r["color"]) };
    if let Some(hl) = rgba(&r["hl"]) {
        inner += &format!("<a:highlight>{hl}</a:highlight>");
    }
    let font = esc(r["font"].as_str().unwrap_or("Arial"));
    inner += &format!("<a:latin typeface=\"{font}\"/><a:ea typeface=\"{font}\"/><a:cs typeface=\"{font}\"/>");
    if let Some(href) = r["href"].as_str() {
        let id = sh.rel("hyperlink", href.to_string());
        inner += &format!("<a:hlinkClick r:id=\"{id}\"/>");
    }
    format!("<a:r><a:rPr{attrs} dirty=\"0\">{inner}</a:rPr><a:t>{}</a:t></a:r>", esc(r["text"].as_str().unwrap_or("")))
}

/// A paragraph: its runs of text, the line spacing, the alignment, the bullet and the space above
/// (paragraphs merged from the same list)
struct Para<'a> {
    runs: &'a [Value],
    lh: f64,
    algn: &'a str,
    bullet: &'a Value,
    gap: f64,
}

fn paragraph(sh: &mut Sheet, p: &Para) -> String {
    let mut ppr = format!("<a:lnSpc><a:spcPts val=\"{}\"/></a:lnSpc>", sh.pt100(p.lh).max(100));
    if p.gap > 0.5 {
        ppr += &format!("<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>", sh.pt100(p.gap));
    }
    let mut margins = String::new();
    if p.bullet.is_null() {
        ppr += "<a:buNone/>";
    } else {
        let b = p.bullet;
        let indent = sh.e(num(b, "indent"));
        margins = format!(" marL=\"{indent}\" indent=\"-{indent}\"");
        if let Some(c) = rgba(&b["color"]) {
            ppr += &format!("<a:buClr>{c}</a:buClr>");
        }
        ppr += &match b["type"].as_str().unwrap_or("disc") {
            "decimal" => "<a:buAutoNum type=\"arabicPeriod\"/>".to_string(),
            "lower-alpha" | "lower-latin" => "<a:buAutoNum type=\"alphaLcPeriod\"/>".into(),
            "upper-alpha" | "upper-latin" => "<a:buAutoNum type=\"alphaUcPeriod\"/>".into(),
            t => format!("<a:buFont typeface=\"Arial\"/><a:buChar char=\"{}\"/>", match t { "circle" => "◦", "square" => "▪", _ => "•" }),
        };
    }
    let runs: String = p.runs.iter().map(|r| run(sh, r)).collect();
    let end = p.runs.first().map(|r| format!("<a:endParaRPr lang=\"{}\" sz=\"{}\" dirty=\"0\"/>", sh.lang, sh.pt100(num(r, "size")).clamp(100, 400_000))).unwrap_or_default();
    format!("<a:p><a:pPr algn=\"{}\"{margins}>{ppr}</a:pPr>{runs}{end}</a:p>", p.algn)
}

fn paras(it: &Value) -> Vec<Para<'_>> {
    // A merged box (see `merge`) carries its paragraphs with their settings
    if let Some(blocks) = it["blocks"].as_array() {
        return blocks.iter().flat_map(paras).collect();
    }
    let lh = num(it, "lh");
    let algn = it["algn"].as_str().unwrap_or("l");
    it["paras"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, runs)| Para {
            runs: runs.as_array().map_or(&[], Vec::as_slice),
            lh,
            algn,
            bullet: &it["bullet"],
            gap: if i == 0 { num(it, "gap") } else { 0.0 },
        })
        .collect()
}

fn text(sh: &mut Sheet, it: &Value) -> String {
    let (mut x, y, mut w, h) = (num(it, "x"), num(it, "y"), num(it, "w"), num(it, "h"));
    let wrap = it["wrap"].as_bool() == Some(true);
    // A little margin: font metrics differ from one application to the next; the line breaks
    // are explicit runs: the margin adds none
    let extra = if wrap { w * 0.03 + 2.0 } else { 0.0 };
    match it["algn"].as_str() {
        Some("ctr") => x -= extra / 2.0,
        Some("r") => x -= extra,
        _ => {}
    }
    w += extra;
    let indent = it["blocks"].as_array().and_then(|b| b.first()).unwrap_or(it)["bullet"]["indent"].as_f64().unwrap_or(0.0);
    x -= indent;
    w += indent;
    let body: String = paras(it).iter().map(|p| paragraph(sh, p)).collect();
    let wrap = if wrap { "square" } else { "none" };
    let text = format!(
        "<p:txBody><a:bodyPr wrap=\"{wrap}\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"ctr\" rtlCol=\"0\"><a:noAutofit/></a:bodyPr>\
<a:lstStyle/>{body}</p:txBody>"
    );
    sp(sh, Sp { name: "Text", frame: (x, y, w, h), geometry: preset("rect", &[]), paint: "<a:noFill/>".into(), text: Some(text) })
}

/// `data:image/png;base64,…` → content and extension
fn data_url(src: &str) -> Option<(Vec<u8>, &'static str)> {
    let (head, data) = src.split_once(',')?;
    let ext = if head.contains("image/png") { "png" } else if head.contains("image/jpeg") { "jpeg" } else if head.contains("image/gif") { "gif" } else { return None };
    Some((STANDARD.decode(data).ok()?, ext))
}

fn picture(sh: &mut Sheet, it: &Value) -> String {
    let Some((bytes, ext)) = it["src"].as_str().and_then(data_url) else { return String::new() };
    let (x, y, w, h) = (num(it, "x"), num(it, "y"), num(it, "w"), num(it, "h"));
    if w <= 0.0 || h <= 0.0 {
        return String::new();
    }
    let rid = sh.image(bytes, ext);
    let id = sh.id();
    format!(
        "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"Picture {id}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>\
<p:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>\
<p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
        xfrm(sh, x, y, w, h)
    )
}

/// The bounding box of an object, in pixels: (x0, y0, x1, y1)
fn bounds(it: &Value) -> Option<(f64, f64, f64, f64)> {
    match it["t"].as_str()? {
        "group" => it["items"].as_array()?.iter().filter_map(bounds).reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        "path" => it["d"]
            .as_array()?
            .iter()
            .filter_map(Value::as_array)
            .flat_map(|c| c[1..].chunks(2).filter_map(|p| Some((p.first()?.as_f64()?, p.get(1)?.as_f64()?))).collect::<Vec<_>>())
            .fold(None, |acc: Option<(f64, f64, f64, f64)>, (x, y)| Some(acc.map_or((x, y, x, y), |a| (a.0.min(x), a.1.min(y), a.2.max(x), a.3.max(y))))),
        _ => {
            let (x, y) = (num(it, "x"), num(it, "y"));
            Some((x, y, x + num(it, "w"), y + num(it, "h")))
        }
    }
}

fn group(sh: &mut Sheet, it: &Value) -> String {
    let items = merge(it["items"].as_array().cloned().unwrap_or_default());
    let Some((x0, y0, x1, y1)) = bounds(it) else { return String::new() };
    let children: String = items.iter().map(|c| shape(sh, c)).collect();
    if children.is_empty() {
        return String::new();
    }
    let id = sh.id();
    let (x, y, w, h) = (sh.e(x0), sh.e(y0), sh.e(x1 - x0).max(1), sh.e(y1 - y0).max(1));
    format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"{id}\" name=\"{} {id}\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
<p:grpSpPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/><a:chOff x=\"{x}\" y=\"{y}\"/><a:chExt cx=\"{w}\" cy=\"{h}\"/></a:xfrm></p:grpSpPr>{children}</p:grpSp>",
        esc(it["name"].as_str().unwrap_or("Group"))
    )
}

/// Merges into one text box the lines and the bullets surveyed one by one (lines of a code block,
/// items of a list): same left edge, one under the other, with nothing painted between them that
/// covers them
fn merge(items: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for it in items {
        if it["t"] == "text"
            && let Some(at) = mergeable(&out, &it) {
                let prev = &mut out[at];
                let (py, ph) = (num(prev, "y"), num(prev, "h"));
                let mut block = it.clone();
                block["gap"] = (num(&it, "y") - (py + ph)).max(0.0).into();
                if prev["blocks"].is_null() {
                    let first = prev.clone();
                    prev["blocks"] = Value::Array(vec![first]);
                }
                if let Some(blocks) = prev["blocks"].as_array_mut() {
                    blocks.push(block);
                }
                let (x0, x1) = (num(prev, "x").min(num(&it, "x")), (num(prev, "x") + num(prev, "w")).max(num(&it, "x") + num(&it, "w")));
                prev["x"] = x0.into();
                prev["w"] = (x1 - x0).into();
                prev["h"] = (num(&it, "y") + num(&it, "h") - py).into();
                continue;
            }
        out.push(it);
    }
    out
}

fn mergeable(out: &[Value], it: &Value) -> Option<usize> {
    let (x, y, w, h) = (num(it, "x"), num(it, "y"), num(it, "w"), num(it, "h"));
    for (i, prev) in out.iter().enumerate().rev().take(60) {
        if prev["t"] != "text" {
            continue;
        }
        let (px, py, pw, ph) = (num(prev, "x"), num(prev, "y"), num(prev, "w"), num(prev, "h"));
        let gap = y - (py + ph);
        let same_side = (px - x).abs() < 1.5 && prev["algn"] == it["algn"] && prev["wrap"] == it["wrap"];
        let near = gap > -1.5 && gap < num(it, "lh").max(num(prev, "lh")) * 1.6;
        let width = !it["wrap"].as_bool().unwrap_or(false) || (pw - w).abs() < 2.0;
        let last_bullet = prev["blocks"].as_array().and_then(|b| b.last()).unwrap_or(prev)["bullet"].clone();
        if same_side && near && width && last_bullet == it["bullet"] {
            // Nothing painted between the two may touch the merged box
            let (u0, u1) = (py, y + h);
            let clear = out[i + 1..].iter().all(|o| match bounds(o) {
                Some((a, b, c, d)) => c <= px.min(x) || a >= (px + pw).max(x + w) || d <= u0 || b >= u1,
                None => true,
            });
            return clear.then_some(i);
        }
    }
    None
}

