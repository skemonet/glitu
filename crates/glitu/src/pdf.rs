//! A PDF made of screenshots (export `--clicks`): one page per image, at the slide's size, an
//! outline with the title of each slide, and page labels: the pages of one slide carry its number,
//! which presentation viewers (pdfpc) read as the overlays of one slide. The PNG data goes into
//! the PDF as is (FlateDecode with the PNG predictor), without recompression.

/// A page: the image, the title of its slide and the slide's number
pub struct Page {
    pub png: Vec<u8>,
    pub title: String,
    pub number: usize,
}

/// Pages of `width` × `height` points
pub fn from_images(pages: &[Page], width: f64, height: f64, title: &str) -> Result<Vec<u8>, String> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    // 1: catalog, 2: page tree, 3: outline; then image, content, page for each one
    let n = pages.len();
    let page_id = |i: usize| 4 + 3 * i + 2;
    // a label where the slide changes; the following pages keep it (no numbering style)
    let labels: String = pages
        .iter()
        .enumerate()
        .filter(|(i, p)| *i == 0 || pages[i - 1].number != p.number)
        .map(|(i, p)| format!("{i} << /P ({}) >> ", p.number))
        .collect();
    objects.push(
        format!("<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R /PageMode /UseOutlines /PageLabels << /Nums [{labels}] >> >>")
            .into_bytes(),
    );
    let kids: String = (0..n).map(|i| format!("{} 0 R ", page_id(i))).collect();
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").into_bytes());
    // Outline: one entry per titled slide, the first page of each title
    let mut entries: Vec<(usize, &str)> = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        let t = p.title.as_str();
        if !t.is_empty() && entries.last().is_none_or(|(_, last)| *last != t) {
            entries.push((i, t));
        }
    }
    let outline_base = 4 + 3 * n;
    if entries.is_empty() {
        objects.push(b"<< /Type /Outlines /Count 0 >>".to_vec());
    } else {
        objects.push(
            format!(
                "<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {} >>",
                outline_base,
                outline_base + entries.len() - 1,
                entries.len()
            )
            .into_bytes(),
        );
    }
    for p in pages {
        let (w, h, idat) = rgb_idat(&p.png)?;
        let mut img = format!(
            "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 \
/Filter /FlateDecode /DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns {w} >> /Length {} >>\nstream\n",
            idat.len()
        )
        .into_bytes();
        img.extend(idat);
        img.extend(b"\nendstream");
        let id = objects.len() + 1;
        objects.push(img);
        let draw = format!("q {width:.2} 0 0 {height:.2} 0 0 cm /Im Do Q");
        objects.push(format!("<< /Length {} >>\nstream\n{draw}\nendstream", draw.len()).into_bytes());
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width:.2} {height:.2}] /Resources << /XObject << /Im {id} 0 R >> >> /Contents {} 0 R >>",
                id + 1
            )
            .into_bytes(),
        );
    }
    for (k, (page, t)) in entries.iter().enumerate() {
        let mut o = format!("<< /Title {} /Parent 3 0 R /Dest [{} 0 R /Fit]", text(t), page_id(*page));
        if k > 0 {
            o += &format!(" /Prev {} 0 R", outline_base + k - 1)
        }
        if k + 1 < entries.len() {
            o += &format!(" /Next {} 0 R", outline_base + k + 1)
        }
        objects.push((o + " >>").into_bytes());
    }
    let info = objects.len() + 1;
    objects.push(format!("<< /Title {} /Producer (glitu) >>", text(title)).into_bytes());

    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).into_bytes());
        out.extend(o);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).into_bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").into_bytes());
    }
    out.extend(
        format!("trailer\n<< /Size {} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1)
            .into_bytes(),
    );
    Ok(out)
}

/// A PDF string in UTF-16 (accented titles)
fn text(s: &str) -> String {
    let mut hex = String::from("<FEFF");
    for u in s.encode_utf16() {
        hex += &format!("{u:04X}");
    }
    hex + ">"
}

/// PNG → (width, height, IDAT data) in non-interlaced 8-bit RGB, re-encoded if needed
fn rgb_idat(png: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let dec = png::Decoder::new(std::io::Cursor::new(png));
    let reader = dec.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    let direct = info.color_type == png::ColorType::Rgb && info.bit_depth == png::BitDepth::Eight && !info.interlaced;
    let (w, h) = (info.width, info.height);
    let data = if direct { png.to_vec() } else { crate::raster::encode(&crate::raster::decode(png)?)? };
    // Chunks after the signature: length, type, data, CRC
    let mut idat = Vec::new();
    let mut at = 8;
    while at + 8 <= data.len() {
        let len = u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        let chunk = data.get(at + 8..at + 8 + len).ok_or("truncated PNG")?;
        if &data[at + 4..at + 8] == b"IDAT" {
            idat.extend(chunk);
        }
        at += 12 + len;
    }
    Ok((w, h, idat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_per_image_and_an_outline() {
        let mut png = Vec::new();
        let mut enc = png::Encoder::new(&mut png, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&[255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        let page = |png: &[u8], title: &str, number| Page { png: png.to_vec(), title: title.into(), number };
        let pages = [page(&png, "Été", 1), page(&png, "Été", 1), page(&png, "", 2)];
        let pdf = from_images(&pages, 960.0, 540.0, "T").unwrap();
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.starts_with("%PDF-1.7"));
        assert_eq!(s.matches("/Type /Page ").count(), 3);
        assert!(s.contains("/Count 1 >>") && s.contains("<FEFF00C9007400E9>"));
        // the two pages of slide 1 share its label
        assert!(s.contains("/PageLabels << /Nums [0 << /P (1) >> 2 << /P (2) >> ] >>"));
        // without titles, an empty outline
        let pdf = from_images(&[page(&png_rgb(), "", 1)], 960.0, 540.0, "T").unwrap();
        assert!(String::from_utf8_lossy(&pdf).contains("<< /Type /Outlines /Count 0 >>"));
        assert!(from_images(&[page(b"not a png", "", 1)], 1.0, 1.0, "T").is_err());
    }

    fn png_rgb() -> Vec<u8> {
        crate::raster::encode(&crate::raster::Rgb { width: 1, height: 1, pixels: vec![1, 2, 3] }).unwrap()
    }
}
