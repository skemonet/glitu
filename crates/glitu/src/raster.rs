//! PNG images as RGB pixels (8 bits, three bytes per pixel), to compare screenshots, recognise a
//! plain backdrop or slip a screenshot into a PDF.

/// An image: width, height and RGB pixels row by row
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// PNG (any depth, with or without transparency, which is ignored) → RGB
pub fn decode(png: &[u8]) -> Result<Rgb, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let n = info.color_type.samples();
    let pixels = buf[..info.buffer_size()].chunks(n).flat_map(|p| if n >= 3 { [p[0], p[1], p[2]] } else { [p[0]; 3] }).collect();
    Ok(Rgb { width: info.width, height: info.height, pixels })
}

pub fn encode(img: &Rgb) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgb);
    enc.write_header().and_then(|mut w| w.write_image_data(&img.pixels)).map_err(|e| e.to_string())?;
    Ok(out)
}

/// The single colour of the image, if it has only one
pub fn uniform(img: &Rgb) -> Option<[u8; 3]> {
    let first = [*img.pixels.first()?, *img.pixels.get(1)?, *img.pixels.get(2)?];
    img.pixels.chunks(3).all(|p| p == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_uniform_colour() {
        let img = Rgb { width: 2, height: 1, pixels: vec![9, 8, 7, 9, 8, 7] };
        let back = decode(&encode(&img).unwrap()).unwrap();
        assert_eq!((back.width, back.height, back.pixels.clone()), (2, 1, img.pixels.clone()));
        assert_eq!(uniform(&back), Some([9, 8, 7]));
        assert_eq!(uniform(&Rgb { width: 2, height: 1, pixels: vec![0, 0, 0, 1, 0, 0] }), None);
        assert_eq!(uniform(&Rgb { width: 0, height: 0, pixels: Vec::new() }), None);
        assert!(decode(b"not a png").is_err());
    }
}
