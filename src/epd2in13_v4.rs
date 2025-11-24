/// This file is a driver for the Waveshare 2.13" V4 e-paper display module.
/// It uses the rppal crate for SPI and GPIO access on Raspberry Pi.
/// It supports full, fast, and partial updates, as well as clearing the display
/// and putting the display to sleep.
/// 
/// Copyright (c) 2024 Santiago Saavedra
/// Copyright (c) 2023 Waveshare Team
/// 
/// Original copyright notice from Waveshare:
// # *****************************************************************************
// # * | File        :	  epd2in13_V4.py
// # * | Author      :   Waveshare team
// # * | Function    :   Electronic paper driver
// # * | Info        :
// # *----------------
// # * | This version:   V1.0
// # * | Date        :   2023-06-25
// # # | Info        :   python demo
// # -----------------------------------------------------------------------------
// # Permission is hereby granted, free of charge, to any person obtaining a copy
// # of this software and associated documnetation files (the "Software"), to deal
// # in the Software without restriction, including without limitation the rights
// # to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// # copies of the Software, and to permit persons to  whom the Software is
// # furished to do so, subject to the following conditions:
// #
// # The above copyright notice and this permission notice shall be included in
// # all copies or substantial portions of the Software.
// #
// # THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// # IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// # FITNESS OR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// # AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// # LIABILITY WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// # OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// # THE SOFTWARE.


use embedded_graphics::pixelcolor::BinaryColor;
use rppal::{
    gpio::{Gpio, InputPin, OutputPin},
    spi::{Bus, Mode, SlaveSelect, Spi},
};
use std::{thread::sleep, time::Duration};
use thiserror::Error;

/// Pin assignments for the panel, using BCM numbering.
#[derive(Debug, Clone, Copy)]
pub struct EpdPins {
    pub busy: u8,
    pub dc: u8,
    pub rst: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum UpdateMode {
    Normal,
    Fast,
    Partial,
}

#[derive(Debug, Error)]
pub enum EpdError {
    #[error("SPI error: {0}")]
    Spi(#[from] rppal::spi::Error),
    #[error("GPIO error: {0}")]
    Gpio(#[from] rppal::gpio::Error),
    #[error("buffer length mismatch: expected {expected} bytes, got {actual}")]
    BufferSize { expected: usize, actual: usize },
}

pub struct Epd2in13V4 {
    spi: Spi,
    busy: InputPin,
    dc: OutputPin,
    rst: OutputPin,
    bytes_per_row: usize,
}

impl Epd2in13V4 {
    pub const WIDTH: u16 = 122;
    pub const HEIGHT: u16 = 250;

    /// Create a driver with the default SPI bus (SPI0, CE0) at 4 MHz.
    pub fn new(pins: EpdPins) -> Result<Self, EpdError> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 4_000_000, Mode::Mode0)?;
        Self::with_spi(spi, pins)
    }

    /// Create a driver using an already configured SPI bus.
    pub fn with_spi(spi: Spi, pins: EpdPins) -> Result<Self, EpdError> {
        let gpio = Gpio::new()?;
        let busy = gpio.get(pins.busy)?.into_input();
        let dc = gpio.get(pins.dc)?.into_output();
        let rst = gpio.get(pins.rst)?.into_output();
        let bytes_per_row = ((Self::WIDTH as usize) + 7) / 8;
        Ok(Self {
            spi,
            busy,
            dc,
            rst,
            bytes_per_row,
        })
    }

    pub fn init(&mut self) -> Result<(), EpdError> {
        self.reset()?;
        self.wait_until_idle();
        self.command(0x12)?; // SWRESET
        self.wait_until_idle();

        self.command_data(0x01, &[0xF9, 0x00, 0x00])?; // driver output control
        self.command_data(0x11, &[0x03])?; // data entry mode

        self.set_window(0, 0, Self::WIDTH - 1, Self::HEIGHT - 1)?;
        self.set_cursor(0, 0)?;

        self.command_data(0x3C, &[0x05])?; // border waveform
        self.command_data(0x21, &[0x00, 0x80])?; // display update control

        self.command_data(0x18, &[0x80])?; // enable internal temp sensor
        self.wait_until_idle();

        Ok(())
    }

    pub fn init_fast(&mut self) -> Result<(), EpdError> {
        self.reset()?;
        self.command(0x12)?;
        self.wait_until_idle();

        self.command_data(0x18, &[0x80])?;
        self.command_data(0x11, &[0x03])?;
        self.set_window(0, 0, Self::WIDTH - 1, Self::HEIGHT - 1)?;
        self.set_cursor(0, 0)?;

        self.command_data(0x22, &[0xB1])?;
        self.command(0x20)?;
        self.wait_until_idle();

        self.command_data(0x1A, &[0x64, 0x00])?;
        self.command_data(0x22, &[0x91])?;
        self.command(0x20)?;
        self.wait_until_idle();
        Ok(())
    }

    pub fn clear(&mut self, color: BinaryColor) -> Result<(), EpdError> {
        let fill = if color == BinaryColor::On { 0x00 } else { 0xFF };
        self.command(0x24)?;
        let line = vec![fill; self.bytes_per_row];
        for _ in 0..Self::HEIGHT {
            self.data(&line)?;
        }
        self.turn_on_display(UpdateMode::Normal)
    }

    pub fn display(&mut self, image: &[u8]) -> Result<(), EpdError> {
        self.write_image(0x24, image)?;
        self.turn_on_display(UpdateMode::Normal)
    }

    pub fn display_fast(&mut self, image: &[u8]) -> Result<(), EpdError> {
        self.write_image(0x24, image)?;
        self.turn_on_display(UpdateMode::Fast)
    }

    pub fn display_base(&mut self, image: &[u8]) -> Result<(), EpdError> {
        self.write_image(0x24, image)?;
        self.write_image(0x26, image)?;
        self.turn_on_display(UpdateMode::Normal)
    }

    pub fn display_partial(&mut self, image: &[u8]) -> Result<(), EpdError> {
        self.reset()?; // partial updates need a short reset
        self.command_data(0x3C, &[0x80])?;
        self.command_data(0x01, &[0xF9, 0x00, 0x00])?;
        self.command_data(0x11, &[0x03])?;
        self.set_window(0, 0, Self::WIDTH - 1, Self::HEIGHT - 1)?;
        self.set_cursor(0, 0)?;

        self.write_image(0x24, image)?;
        self.turn_on_display(UpdateMode::Partial)
    }

    pub fn sleep(&mut self) -> Result<(), EpdError> {
        self.command_data(0x10, &[0x01])?;
        sleep(Duration::from_millis(100));
        Ok(())
    }

    fn write_image(&mut self, command: u8, image: &[u8]) -> Result<(), EpdError> {
        let expected = self.bytes_per_row * Self::HEIGHT as usize;
        if image.len() != expected {
            return Err(EpdError::BufferSize {
                expected,
                actual: image.len(),
            });
        }
        self.command(command)?;
        self.data(image)?;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), EpdError> {
        self.rst.set_high();
        sleep(Duration::from_millis(20));
        self.rst.set_low();
        sleep(Duration::from_millis(2));
        self.rst.set_high();
        sleep(Duration::from_millis(20));
        Ok(())
    }

    fn wait_until_idle(&mut self) {
        while self.busy.is_high() {
            sleep(Duration::from_millis(10));
        }
        sleep(Duration::from_millis(10));
    }

    fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), EpdError> {
        self.command_data(0x44, &[(x_start / 8) as u8, (x_end / 8) as u8])?;
        self.command_data(
            0x45,
            &[
                (y_start & 0xFF) as u8,
                (y_start >> 8) as u8,
                (y_end & 0xFF) as u8,
                (y_end >> 8) as u8,
            ],
        )?;
        Ok(())
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), EpdError> {
        self.command_data(0x4E, &[(x / 8) as u8])?;
        self.command_data(0x4F, &[(y & 0xFF) as u8, (y >> 8) as u8])?;
        Ok(())
    }

    fn turn_on_display(&mut self, mode: UpdateMode) -> Result<(), EpdError> {
        let control = match mode {
            UpdateMode::Normal => 0xF7,
            UpdateMode::Fast => 0xC7,
            UpdateMode::Partial => 0xFF,
        };
        self.command_data(0x22, &[control])?;
        self.command(0x20)?;
        self.wait_until_idle();
        Ok(())
    }

    fn command(&mut self, command: u8) -> Result<(), EpdError> {
        self.dc.set_low();
        self.spi.write(&[command])?;
        Ok(())
    }

    fn data(&mut self, data: &[u8]) -> Result<(), EpdError> {
        self.dc.set_high();
        self.spi.write(data)?;
        Ok(())
    }

    fn command_data(&mut self, command: u8, data: &[u8]) -> Result<(), EpdError> {
        self.command(command)?;
        self.data(data)
    }
}
