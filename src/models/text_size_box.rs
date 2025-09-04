use ab_glyph::{point, Font, GlyphId, OutlinedGlyph, PxScale, ScaleFont};

/// Text frame size
pub struct TextSizeBox {
    pub w: u32,
    pub h: u32,
}

impl TextSizeBox {
    /// Calculate size of text with selected font
    ///
    /// Parameters:
    ///  - text: input string
    ///  - font: font, wow
    ///  - scale: scale of font
    ///
    /// Return: TextSizeBox instance
    pub fn from<T : Font>(text: &str, font: T, scale: PxScale) -> Self {
        let scaled_font = font.as_scaled(scale);
        let w = 0f32;
        let mut last: Option<GlyphId> = None;
        text.chars().map(|char| {
            let glyph_id = scaled_font.glyph_id(char);
            let glyph = glyph_id.with_scale_and_position(scale, point(w, scaled_font.ascent()));
            let advance = scaled_font.h_advance(glyph_id);
            let (w, h) = scaled_font.outline_glyph(glyph)
                .map(|outlined_glyph: OutlinedGlyph| {
                    let w = last.map(|last_glyph| scaled_font.kern(glyph_id, last_glyph)).unwrap_or(0f32);
                    last = Some(glyph_id);
                    (w, outlined_glyph.px_bounds().height())
                })
                .unwrap_or((0f32, 0f32));

            (advance + w, h)
        })
            .fold(_Accumulator::empty(), |mut acc, bbox| *acc.step(bbox))
            .result()
    }
}

#[derive(Copy, Clone)]
struct _Accumulator {
    w: f32,
    h: f32,
    last_w: f32,
}

impl _Accumulator {
    fn empty() -> Self {
        Self {
            w: 0f32,
            h: 0f32,
            last_w: 0f32,
        }
    }

    fn step(&mut self, (w, h): (f32, f32)) -> &Self {
        self.h = self.h.max(h);
        self.w = self.w + w;
        self
    }

    fn result(&self) -> TextSizeBox {
        TextSizeBox {
            w: (self.w + self.last_w) as u32,
            h: self.h as u32,
        }
    }
}
