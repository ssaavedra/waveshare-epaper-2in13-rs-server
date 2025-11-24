use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::Text,
};
use rpi_einkserver_rs::{Epd2in13V4, EpdPins, MonoImage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default Waveshare HAT pins (BCM numbering): BUSY=24, RST=17, DC=25.
    let pins = EpdPins {
        busy: 24,
        dc: 25,
        rst: 17,
    };

    let mut epd = Epd2in13V4::new(pins)?;
    epd.init()?;

    let mut fb = MonoImage::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32);
    fb.clear(BinaryColor::On); // white background

    let border = Rectangle::new(
        Point::new(0, 0),
        Size::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32),
    );
    border
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
        .draw(&mut fb)?;

    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
    Text::new("Waveshare 2.13\" V4", Point::new(6, 20), style).draw(&mut fb)?;
    Text::new("Rust + rppal demo", Point::new(6, 34), style).draw(&mut fb)?;
    Text::new("Hello World!", Point::new(6, 48), style).draw(&mut fb)?;

    // Circle::new(Point::new(20, 70), 30)
    //     .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
    //     .draw(&mut fb)?;
    // Rectangle::new(Point::new(70, 60), Size::new(40, 40))
    //     .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
    //     .draw(&mut fb)?;

    epd.display(fb.data())?;
    epd.sleep()?;

    Ok(())
}
