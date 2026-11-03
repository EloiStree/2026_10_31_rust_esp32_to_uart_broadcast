use core::convert::Infallible;

use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, OriginDimensions, Pixel, Point, Size},
    text::{Baseline, Text},
};

pub const SSD1306_SDA_PIN: u8 = 7;
pub const SSD1306_SCL_PIN: u8 = 8;
pub const SSD1306_I2C_ADDRESSES: [u8; 2] = [0x3C, 0x3D];

/// Helper for drawing simple patterns on a monochrome SSD1306 display.
pub struct StructSetSSD1306 {
    pixels: alloc::boxed::Box<[u8]>,
    i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>,
    address: u8,
}

#[allow(
    dead_code,
    reason = "Optional display patterns are useful for hardware diagnostics"
)]
impl StructSetSSD1306 {
    /// Takes ownership of a configured I2C bus and initializes the display.
    pub fn new(
        i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>,
    ) -> Result<Self, esp_hal::i2c::master::Error> {
        let mut i2c = i2c;
        // Probe the two addresses an SSD1306 module can be strapped to.
        let mut address = SSD1306_I2C_ADDRESSES[0];
        let mut probe = Ok(());
        for candidate in SSD1306_I2C_ADDRESSES {
            probe = i2c.write(candidate, &[0x00, 0xAE]);
            if probe.is_ok() {
                address = candidate;
                break;
            }
        }
        probe?;
        let mut s = Self {
            pixels: alloc::vec![0; 1024].into_boxed_slice(),
            i2c,
            address,
        };
        // Standard SSD1306 128x64 init sequence (control byte 0x00 = commands).
        let init: [u8; 25] = [
            0x00, 0xAE, 0xD5, 0x80, 0xA8, 0x3F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0x00, 0xA1,
            0xC8, 0xDA, 0x12, 0x81, 0xCF, 0xD9, 0xF1, 0xDB, 0x40, 0xA4, 0xA6,
        ];
        s.i2c.write(s.address, &init)?;
        s.i2c.write(s.address, &[0x00, 0xAF])?;
        Ok(s)
    }

    fn put(&mut self, p: Point, c: BinaryColor) {
        if p.x >= 0 && p.y >= 0 && (p.x as usize) < 128 && (p.y as usize) < 64 {
            let x = p.x as usize;
            let y = p.y as usize;
            let index = (y / 8) * 128 + x;
            let bit = 1 << (y % 8);
            match c {
                BinaryColor::On => self.pixels[index] |= bit,
                BinaryColor::Off => self.pixels[index] &= !bit,
            }
        }
    }

    /// Pushes the stored pixel array to the display.
    pub fn refresh(&mut self) -> Result<(), esp_hal::i2c::master::Error> {
        for page in 0..8usize {
            // Set page and column start.
            self.i2c
                .write(self.address, &[0x00, 0xB0 | page as u8, 0x00, 0x10])?;
            let mut buf = [0u8; 129];
            buf[0] = 0x40; // control byte: data
            for x in 0..128usize {
                buf[x + 1] = self.pixels[page * 128 + x];
            }
            self.i2c.write(self.address, &buf)?;
        }
        Ok(())
    }

    // pub fn set_display_with_1024_bytes_array(&mut self, data: &[u8; 1024]) -> Result<(), esp_hal::i2c::master::Error> {
    //     // SSD1306 updates display memory in 8-pixel columns, so only transmit
    //     // changed bytes, grouping adjacent changes into runs per page.
    //     for page in 0..8usize {
    //         let page_start = page * 128;
    //         let mut column = 0usize;

    //         while column < 128 {
    //             while column < 128
    //                 && self.pixels[page_start + column] == data[page_start + column]
    //             {
    //                 column += 1;
    //             }
    //             if column == 128 {
    //                 break;
    //             }

    //             let start = column;
    //             while column < 128
    //                 && self.pixels[page_start + column] != data[page_start + column]
    //             {
    //                 column += 1;
    //             }

    //             self.i2c.write(
    //                 SSD1306_I2C_ADDRESS,
    //                 &[
    //                     0x00,
    //                     0xB0 | page as u8,
    //                     (start & 0x0F) as u8,
    //                     0x10 | ((start >> 4) as u8),
    //                 ],
    //             )?;

    //             let mut buf = [0u8; 129];
    //             buf[0] = 0x40;
    //             let len = column - start;
    //             buf[1..=len].copy_from_slice(&data[page_start + start..page_start + column]);
    //             self.i2c.write(SSD1306_I2C_ADDRESS, &buf[..=len])?;
    //         }
    //     }

    //     self.pixels.copy_from_slice(data);
    //     Ok(())
    // }

    pub fn set_white(&mut self) -> Result<(), esp_hal::i2c::master::Error> {
        self.pixels.fill(u8::MAX);
        self.refresh()
    }

    pub fn set_black(&mut self) -> Result<(), esp_hal::i2c::master::Error> {
        self.pixels.fill(0);
        self.refresh()
    }

    pub fn set_grid(&mut self, spacing: u32) -> Result<(), esp_hal::i2c::master::Error> {
        let spacing = spacing.max(1) as usize;
        for y in 0usize..64 {
            for x in 0usize..128 {
                if x.is_multiple_of(spacing) || y.is_multiple_of(spacing) {
                    self.put(Point::new(x as i32, y as i32), BinaryColor::On);
                } else {
                    self.put(Point::new(x as i32, y as i32), BinaryColor::Off);
                }
            }
        }
        self.refresh()
    }

    pub fn set_text(
        &mut self,
        text: &str,
        position: Point,
    ) -> Result<(), esp_hal::i2c::master::Error> {
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        Text::with_baseline(text, position, style, Baseline::Top)
            .draw(self)
            .unwrap();
        self.refresh()
    }

    pub fn set_random(&mut self) -> Result<(), esp_hal::i2c::master::Error> {
        for y in 0usize..64 {
            for x in 0usize..128 {
                let on = {
                    let mut n = 0x1234_5678u32
                        ^ (x as u32).wrapping_mul(0x9E37_79B9)
                        ^ (y as u32).wrapping_mul(0x85EB_CA6B);
                    n ^= n >> 16;
                    n = n.wrapping_mul(0x7FEB_352D);
                    n ^= n >> 15;
                    n & 1 != 0
                };
                let color = if on {
                    BinaryColor::On
                } else {
                    BinaryColor::Off
                };
                self.put(Point::new(x as i32, y as i32), color);
            }
        }

        self.refresh()
    }

    pub fn set_from_1024_bytes_array_top_left_to_down_right(
        &mut self,
        data: &[u8; 1024],
    ) -> Result<(), esp_hal::i2c::master::Error> {
        for y in 0usize..64 {
            for x in 0usize..128 {
                let byte_index = (y / 8) * 128 + x;
                let bit_index = y % 8;
                let on = (data[byte_index] >> bit_index) & 1 != 0;
                let color = if on {
                    BinaryColor::On
                } else {
                    BinaryColor::Off
                };
                self.put(Point::new(x as i32, y as i32), color);
            }
        }
        self.refresh()
    }
}

impl OriginDimensions for StructSetSSD1306 {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}

impl DrawTarget for StructSetSSD1306 {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.put(point, color);
        }
        Ok(())
    }
}
