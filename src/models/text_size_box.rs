use rusttype::{Font, Point, Rect, Scale};
use std::cmp::max;

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
    pub fn from(text: &str, font: &Font, scale: Scale) -> Self {
        font.layout(text, scale, Point::default())
            .filter_map(|pg| pg.pixel_bounding_box())
            .fold(_Accumulator::empty(), |mut acc, bbox| *acc.step(bbox))
            .result()
    }
}

#[derive(Copy, Clone)]
struct _Accumulator {
    w: i32,
    h: i32,
    last_w: i32,
}

impl _Accumulator {
    fn empty() -> Self {
        Self {
            w: 0,
            h: 0,
            last_w: 0,
        }
    }

    fn step(&mut self, bbox: Rect<i32>) -> &Self {
        self.last_w = bbox.width();
        self.h = max(self.h, bbox.max.y + bbox.height());
        self.w = bbox.min.x;
        self
    }

    fn result(&self) -> TextSizeBox {
        TextSizeBox {
            w: ((self.w + self.last_w) as f32) as u32,
            h: self.h as u32,
        }
    }
}
