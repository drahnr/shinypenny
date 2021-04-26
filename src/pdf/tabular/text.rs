use super::super::constants::TTF_REGULAR;
use super::*;
use crate::errors::*;
use printpdf::*;

use harfbuzz_rs as harf;

///
pub(crate) fn text_width(text: &str, font_as_bytes: &[u8], size: i32) -> Result<Pt> {
    let index = 0; //< face index in the font file
    let face = harf::Face::from_bytes(font_as_bytes, index);
    let mut font = harf::Font::new(face);

    const HIGH_PRECISION: i32 = 256i32;
    font.set_scale(size * HIGH_PRECISION, size * HIGH_PRECISION); // for higher precision alginment
    let font = font;

    let buffer = harf::UnicodeBuffer::new().add_str(text);
    let output = harf::shape(&font, buffer, &[]);

    // The results of the shaping operation are stored in the `output` buffer.

    let positions = output.get_glyph_positions();

    // iterate over the shaped glyphs
    let mut x = 0i32;
    for position in positions {
        x += position.x_advance;
    }

    // https://stackoverflow.com/questions/50292283/units-used-by-hb-position-t-in-harfbuzz
    // https://github.com/harfbuzz/harfbuzz/issues/2714

    // rescale to the actual font size
    let length = Pt(x as f64 / HIGH_PRECISION as f64);

    Ok(length + Pt(size as f64) * 0.25f64) // some extra padding, right now this is only used for aligning text to the right
}

pub(crate) fn text(
    layer: &PdfLayerReference,
    mut anchor: Point,
    text: &str,
    font: &IndirectFontRef,
    size: i32,
    align: Alignment,
) -> Result<Pt> {
    let length = text_width(text, TTF_REGULAR, size)?;

    anchor.x = match align {
        Alignment::Left => anchor.x,
        Alignment::Right => anchor.x - length,
        Alignment::Center => anchor.x - length / 2.0f64,
    };

    layer.use_text(
        text,
        size as i64 as f64,
        Mm::from(anchor.x),
        Mm::from(anchor.y),
        &font,
    );
    Ok(length)
}
