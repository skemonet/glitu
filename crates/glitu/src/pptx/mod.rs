//! Writing a PowerPoint file (.pptx) from what `assets/export.js` surveys on screen: each element
//! becomes an independent and editable PowerPoint object (text box, shape, path, group for a
//! diagram, image), laid on the slide's backdrop. The speaker notes go into the notes page. The
//! OOXML package is written by hand: one master, one blank layout, one theme.

mod shapes;

use serde_json::Value;

use crate::zip::Zip;
use shapes::Sheet;

/// A slide: its backdrop, its objects (in page pixels, see `export.js`) and its notes
pub struct Slide {
    pub background: Background,
    pub items: Vec<Value>,
    pub notes: Vec<String>,
}

pub enum Background {
    Color([u8; 3]),
    Png(Vec<u8>),
}

/// Width of the slides, in EMU (13.33 inches, PowerPoint's 16:9 width)
const WIDTH: u64 = 12_192_000;

const NS: &str = "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"";
const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const CT: &str = "application/vnd.openxmlformats-officedocument.";
const GROUP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";
const CLR_MAP: &str = "<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" \
accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>";

/// Text for XML: escaped, without the control characters XML refuses
fn esc(s: &str) -> String {
    glitu_core::html::esc(&s.replace(|c: char| matches!(c as u32, 0..=8 | 11 | 12 | 14..=31), ""))
}

/// `.rels`: (type, target) numbered rId1, rId2…; an `http…` target is external
fn rels(targets: &[(&str, String)]) -> String {
    let mut s = format!("{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    for (i, (kind, target)) in targets.iter().enumerate() {
        let kind = if kind.starts_with("http") { kind.to_string() } else { format!("{REL}{kind}") };
        let mode = if target.contains("://") || target.starts_with("mailto:") { " TargetMode=\"External\"" } else { "" };
        s += &format!("<Relationship Id=\"rId{}\" Type=\"{kind}\" Target=\"{}\"{mode}/>", i + 1, esc(target));
    }
    s + "</Relationships>"
}

// ---------------------------------------------------------------- package

/// Notes page (portrait 7.5 × 10 inches): the slide on top, the notes below
const NOTES_W: u64 = 6_858_000;
const NOTES_H: u64 = 9_144_000;

fn notes_shapes(ratio: f64, notes: &[String], lang: &str) -> String {
    let margin = 685_800;
    let img_w = NOTES_W - 2 * margin;
    let img_h = (img_w as f64 * ratio) as u64;
    let body_y = margin + img_h + 457_200;
    let paras: String = if notes.is_empty() {
        format!("<a:p><a:endParaRPr lang=\"{lang}\"/></a:p>")
    } else {
        notes.iter().map(|p| format!("<a:p><a:r><a:rPr lang=\"{lang}\" dirty=\"0\"/><a:t>{}</a:t></a:r></a:p>", esc(p))).collect()
    };
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Slide image\"/><p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr>\
<p:nvPr><p:ph type=\"sldImg\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{margin}\" y=\"{margin}\"/><a:ext cx=\"{img_w}\" cy=\"{img_h}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:sp>\
<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Notes\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"{margin}\" y=\"{body_y}\"/><a:ext cx=\"{img_w}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>\
<p:txBody><a:bodyPr/><a:lstStyle/>{paras}</p:txBody></p:sp>",
        NOTES_H - body_y - margin
    )
}

const THEME: &str = "<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"glitu\"><a:themeElements>\
<a:clrScheme name=\"glitu\"><a:dk1><a:srgbClr val=\"000000\"/></a:dk1><a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1>\
<a:dk2><a:srgbClr val=\"1F2937\"/></a:dk2><a:lt2><a:srgbClr val=\"E5E7EB\"/></a:lt2>\
<a:accent1><a:srgbClr val=\"22D3EE\"/></a:accent1><a:accent2><a:srgbClr val=\"6366F1\"/></a:accent2>\
<a:accent3><a:srgbClr val=\"10B981\"/></a:accent3><a:accent4><a:srgbClr val=\"F59E0B\"/></a:accent4>\
<a:accent5><a:srgbClr val=\"EF4444\"/></a:accent5><a:accent6><a:srgbClr val=\"8B5CF6\"/></a:accent6>\
<a:hlink><a:srgbClr val=\"0563C1\"/></a:hlink><a:folHlink><a:srgbClr val=\"954F72\"/></a:folHlink></a:clrScheme>\
<a:fontScheme name=\"glitu\"><a:majorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont>\
<a:minorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme>\
<a:fmtScheme name=\"glitu\"><a:fillStyleLst>{FILL}{FILL}{FILL}</a:fillStyleLst>\
<a:lnStyleLst>{LINE}{LINE}{LINE}</a:lnStyleLst>\
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>\
<a:bgFillStyleLst>{FILL}{FILL}{FILL}</a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>";

fn theme() -> String {
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line = format!("<a:ln w=\"9525\">{fill}</a:ln>");
    format!("{HEAD}{}", THEME.replace("{FILL}", fill).replace("{LINE}", &line))
}

/// What the whole file needs to know of the presentation
pub struct Deck<'a> {
    /// Size of the slides, in pixels (the presentation's proportions)
    pub width: u32,
    pub height: u32,
    pub title: &'a str,
    pub author: &'a str,
    /// The presentation's language (`en`, `fr`, `pt-BR`…), for spell checking
    pub lang: &'a str,
}

/// An Office language tag: `fr` → `fr-FR`, `en` → `en-US`, a full tag as is
fn office_lang(lang: &str) -> String {
    match lang {
        "" | "en" => "en-US".into(),
        l if l.contains('-') => l.into(),
        l => format!("{l}-{}", l.to_uppercase()),
    }
}

/// The .pptx file
pub fn write(slides: &[Slide], deck: &Deck) -> Vec<u8> {
    let Deck { width, height, title, author, .. } = *deck;
    let lang = office_lang(deck.lang);
    let (cx, cy) = (WIDTH, WIDTH * height as u64 / width as u64);
    let ratio = height as f64 / width as f64;
    let mut zip = Zip::default();
    let mut images = 0;
    let mut slide_parts = Vec::new();

    for (i, slide) in slides.iter().enumerate() {
        let n = i + 1;
        let mut sh = Sheet::new(width, WIDTH, &lang);
        sh.rel("slideLayout", "../slideLayouts/slideLayout1.xml".into());
        sh.rel("notesSlide", format!("../notesSlides/notesSlide{n}.xml"));
        let bg = match &slide.background {
            Background::Color([r, g, b]) => format!("<a:solidFill><a:srgbClr val=\"{r:02X}{g:02X}{b:02X}\"/></a:solidFill>"),
            Background::Png(png) => {
                let rid = sh.image(png.clone(), "png");
                format!("<a:blipFill dpi=\"0\" rotWithShape=\"1\"><a:blip r:embed=\"{rid}\"/><a:srcRect/><a:stretch><a:fillRect/></a:stretch></a:blipFill>")
            }
        };
        let shapes = shapes::shapes(&mut sh, slide.items.clone());
        let mut rels_list = sh.rels().to_vec();
        for (rel, bytes, ext) in &sh.media {
            images += 1;
            let name = format!("image{images}.{ext}");
            let target = &mut rels_list[rel - 1].1;
            *target = format!("../media/{name}");
            zip.add(&format!("ppt/media/{name}"), bytes.clone());
        }
        slide_parts.push((
            format!(
                "{HEAD}<p:sld {NS}><p:cSld><p:bg><p:bgPr>{bg}<a:effectLst/></p:bgPr></p:bg><p:spTree>{GROUP}{shapes}</p:spTree></p:cSld>\
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"
            ),
            rels(&rels_list),
        ));
    }

    let mut types = format!(
        "{HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/>\
<Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/><Default Extension=\"gif\" ContentType=\"image/gif\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
<Override PartName=\"/docProps/app.xml\" ContentType=\"{CT}extended-properties+xml\"/>"
    );
    for (part, kind) in [
        ("presentation", "presentationml.presentation.main"),
        ("presProps", "presentationml.presProps"),
        ("viewProps", "presentationml.viewProps"),
        ("tableStyles", "presentationml.tableStyles"),
        ("slideMasters/slideMaster1", "presentationml.slideMaster"),
        ("slideLayouts/slideLayout1", "presentationml.slideLayout"),
        ("notesMasters/notesMaster1", "presentationml.notesMaster"),
        ("theme/theme1", "theme"),
        ("theme/theme2", "theme"),
    ] {
        types += &format!("<Override PartName=\"/ppt/{part}.xml\" ContentType=\"{CT}{kind}+xml\"/>");
    }
    for n in 1..=slides.len() {
        types += &format!("<Override PartName=\"/ppt/slides/slide{n}.xml\" ContentType=\"{CT}presentationml.slide+xml\"/>");
        types += &format!("<Override PartName=\"/ppt/notesSlides/notesSlide{n}.xml\" ContentType=\"{CT}presentationml.notesSlide+xml\"/>");
    }
    // [Content_Types].xml first in the archive, by convention
    let mut package = Zip::default();
    package.add("[Content_Types].xml", (types + "</Types>").into_bytes());
    package.append(zip);
    let mut zip = package;

    zip.add("_rels/.rels", rels(&[
        ("officeDocument", "ppt/presentation.xml".into()),
        ("http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties", "docProps/core.xml".into()),
        ("extended-properties", "docProps/app.xml".into()),
    ]).into_bytes());
    zip.add("docProps/core.xml", format!(
        "{HEAD}<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>",
        esc(title), esc(author)
    ).into_bytes());
    zip.add("docProps/app.xml", format!(
        "{HEAD}<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\">\
<Application>glitu</Application><Slides>{}</Slides><Notes>{}</Notes></Properties>",
        slides.len(), slides.len()
    ).into_bytes());

    let mut pres_rels: Vec<(&str, String)> = vec![
        ("slideMaster", "slideMasters/slideMaster1.xml".into()),
        ("notesMaster", "notesMasters/notesMaster1.xml".into()),
        ("theme", "theme/theme1.xml".into()),
        ("presProps", "presProps.xml".into()),
        ("viewProps", "viewProps.xml".into()),
        ("tableStyles", "tableStyles.xml".into()),
    ];
    let first = pres_rels.len() + 1;
    pres_rels.extend((1..=slides.len()).map(|n| ("slide", format!("slides/slide{n}.xml"))));
    let ids: String = (0..slides.len()).map(|i| format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 256 + i, first + i)).collect();
    zip.add("ppt/presentation.xml", format!(
        "{HEAD}<p:presentation {NS} saveSubsetFonts=\"1\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst>\
<p:notesMasterIdLst><p:notesMasterId r:id=\"rId2\"/></p:notesMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst>\
<p:sldSz cx=\"{cx}\" cy=\"{cy}\"/><p:notesSz cx=\"{NOTES_W}\" cy=\"{NOTES_H}\"/></p:presentation>"
    ).into_bytes());
    zip.add("ppt/_rels/presentation.xml.rels", rels(&pres_rels).into_bytes());
    zip.add("ppt/presProps.xml", format!("{HEAD}<p:presentationPr {NS}/>").into_bytes());
    zip.add("ppt/viewProps.xml", format!("{HEAD}<p:viewPr {NS}/>").into_bytes());
    zip.add("ppt/tableStyles.xml", format!(
        "{HEAD}<a:tblStyleLst xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" def=\"{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}\"/>"
    ).into_bytes());
    zip.add("ppt/theme/theme1.xml", theme().into_bytes());
    zip.add("ppt/theme/theme2.xml", theme().into_bytes());

    zip.add("ppt/slideMasters/slideMaster1.xml", format!(
        "{HEAD}<p:sldMaster {NS}><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg><p:spTree>{GROUP}</p:spTree></p:cSld>\
{CLR_MAP}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst></p:sldMaster>"
    ).into_bytes());
    zip.add("ppt/slideMasters/_rels/slideMaster1.xml.rels", rels(&[
        ("slideLayout", "../slideLayouts/slideLayout1.xml".into()),
        ("theme", "../theme/theme1.xml".into()),
    ]).into_bytes());
    zip.add("ppt/slideLayouts/slideLayout1.xml", format!(
        "{HEAD}<p:sldLayout {NS} type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree>{GROUP}</p:spTree></p:cSld>\
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"
    ).into_bytes());
    zip.add("ppt/slideLayouts/_rels/slideLayout1.xml.rels", rels(&[("slideMaster", "../slideMasters/slideMaster1.xml".into())]).into_bytes());
    zip.add("ppt/notesMasters/notesMaster1.xml", format!(
        "{HEAD}<p:notesMaster {NS}><p:cSld><p:spTree>{GROUP}{}</p:spTree></p:cSld>{CLR_MAP}</p:notesMaster>",
        notes_shapes(ratio, &[], &lang)
    ).into_bytes());
    zip.add("ppt/notesMasters/_rels/notesMaster1.xml.rels", rels(&[("theme", "../theme/theme2.xml".into())]).into_bytes());

    for (i, (slide, slide_rels)) in slide_parts.into_iter().enumerate() {
        let n = i + 1;
        zip.add(&format!("ppt/slides/slide{n}.xml"), slide.into_bytes());
        zip.add(&format!("ppt/slides/_rels/slide{n}.xml.rels"), slide_rels.into_bytes());
        zip.add(&format!("ppt/notesSlides/notesSlide{n}.xml"), format!(
            "{HEAD}<p:notes {NS}><p:cSld><p:spTree>{GROUP}{}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>",
            notes_shapes(ratio, &slides[i].notes, &lang)
        ).into_bytes());
        zip.add(&format!("ppt/notesSlides/_rels/notesSlide{n}.xml.rels"), rels(&[
            ("notesMaster", "../notesMasters/notesMaster1.xml".into()),
            ("slide", format!("../slides/slide{n}.xml")),
        ]).into_bytes());
    }
    zip.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_package_holds_editable_shapes_and_notes() {
        let run = |t: &str| json!({ "text": t, "font": "Lato", "size": 24, "b": true, "color": [255, 255, 255, 1] });
        let line = |y: f64, t: &str| json!({ "t": "text", "x": 100, "y": y, "w": 300, "h": 30, "wrap": false, "algn": "l", "lh": 30, "paras": [[run(t)]] });
        let slides = vec![Slide {
            background: Background::Color([0, 0, 0]),
            items: vec![
                json!({ "t": "box", "x": 0, "y": 0, "w": 1280, "h": 60, "fill": { "c": [10, 20, 30, 0.5] }, "r": [8, 8, 8, 8] }),
                line(100.0, "Say <this>"),
                line(130.0, "& that"),
                json!({ "t": "path", "d": [["M", 0, 0], ["C", 10, 0, 20, 10, 20, 20], ["Z"]], "fill": { "c": [1, 2, 3, 1] }, "tail": true, "line": { "w": 2, "c": [9, 9, 9, 1] } }),
            ],
            notes: vec!["A note".into()],
        }];
        let file = write(&slides, &Deck { width: 1280, height: 720, title: "Title", author: "Me", lang: "fr" });
        let text = String::from_utf8_lossy(&file);
        assert!(file.starts_with(b"PK\x03\x04"));
        assert!(text.contains("<p:sldSz cx=\"12192000\" cy=\"6858000\"/>"));
        // The two lines one under the other make a single text box
        assert_eq!(text.matches("txBox=\"1\"").count(), 1);
        assert!(text.contains("Say &lt;this&gt;") && text.contains("&amp; that"));
        assert!(text.contains("prst=\"roundRect\"") && text.contains("<a:alpha val=\"50000\"/>"));
        assert!(text.contains("<a:cubicBezTo>") && text.contains("<a:tailEnd type=\"triangle\""));
        assert!(text.contains("sz=\"1800\""));
        assert!(text.contains("A note") && text.contains("lang=\"fr-FR\""));
        assert_eq!((office_lang("en"), office_lang("pt-BR"), office_lang("de")), ("en-US".into(), "pt-BR".into(), "de-DE".into()));
    }
}
