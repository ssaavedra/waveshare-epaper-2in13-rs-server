use embedded_graphics::{
    draw_target::DrawTarget, geometry::OriginDimensions, pixelcolor::BinaryColor, prelude::*,
};

/// Simple 1-bit framebuffer laid out in the format expected by the Waveshare panel.
pub struct MonoImage {
    width: u32,
    height: u32,
    bytes_per_row: usize,
    data: Vec<u8>,
}

impl MonoImage {
    pub fn new(width: u32, height: u32) -> Self {
        let bytes_per_row = ((width + 7) / 8) as usize;
        let len = bytes_per_row * height as usize;
        Self {
            width,
            height,
            bytes_per_row,
            data: vec![0xFF; len],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Clear the buffer with a single color.
    pub fn clear(&mut self, color: BinaryColor) {
        let fill = if color == BinaryColor::Off {
            0xFF
        } else {
            0x00
        };
        self.data.fill(fill);
    }

    /// Raw byte representation suitable for sending directly to the panel.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: BinaryColor) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y as usize) * self.bytes_per_row + (x as usize / 8);
        let mask = 0x80 >> (x & 0x07);

        match color {
            BinaryColor::Off => self.data[idx] |= mask, // white
            BinaryColor::On => self.data[idx] &= !mask, // black
        }
    }
}

impl OriginDimensions for MonoImage {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl DrawTarget for MonoImage {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x < 0 || coord.y < 0 {
                continue;
            }
            self.set_pixel(coord.x as u32, coord.y as u32, color);
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.clear(color);
        Ok(())
    }
}
