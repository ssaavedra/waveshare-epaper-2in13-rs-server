use clap::Parser;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use rpi_einkserver_rs::{Epd2in13V4, EpdPins, MonoImage};

#[derive(Parser, Debug)]
#[command(
    name = "rpi-einkserver-rs",
    author,
    version,
    about = "Minimal demo for the Waveshare 2.13\" V4 e-Paper HAT"
)]
struct Args {
    /// Text to render (will be wrapped to fit the display)
    #[arg(long)]
    text: Option<String>,

    /// Flash black, then white, then sleep the panel
    #[arg(long)]
    clear: bool,

    #[arg(long, short)]
    fast: bool,

    #[arg(long)]
    noinit: bool,

    #[arg(long)]
    reverse_color: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Default Waveshare HAT pins (BCM numbering): BUSY=24, RST=17, DC=25.
    let pins = EpdPins {
        busy: 24,
        dc: 25,
        cs: 8,
        rst: 17,
    };

    let mut epd = Epd2in13V4::new(pins)?;
    let bg_color = if args.reverse_color {
        BinaryColor::On
    } else {
        BinaryColor::Off
    };

    let fg_color = if args.reverse_color {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    
    if args.noinit {
        println!("Skipping panel initialization as requested.");
    } else if args.fast {
        epd.init_fast()?;
    } else {
        epd.init()?;
    }

    if args.clear {
        epd.clear(bg_color)?;
        epd.sleep()?;
        return Ok(());
    }

    let message = args
        .text
        .as_deref()
        .unwrap_or("Hello from Rust! Pass --text \"your message\" to set custom text.");

    let mut fb = MonoImage::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32);
    // fb.clear(bg_color);

    // Simple border to frame the text area.
    Rectangle::new(
        Point::new(0, 0),
        Size::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(fg_color, 1))
    .draw(&mut fb)?;

    let margin = 6i32;
    let char_width = 10usize;
    let line_height = 20i32; // small extra leading to keep things readable
    let max_chars = ((Epd2in13V4::WIDTH as usize).saturating_sub((margin as usize) * 2)
        / char_width)
        .max(1);
    let max_lines =
        (Epd2in13V4::HEIGHT as usize).saturating_sub((margin as usize) * 2) / line_height as usize;
    let lines = wrap_text(message, max_chars);

    let style = MonoTextStyle::new(&FONT_10X20, fg_color);
    let mut y = margin + 20;
    for line in lines.into_iter().take(max_lines) {
        Text::new(&line, Point::new(margin, y), style).draw(&mut fb)?;
        y += line_height;
    }

    if args.fast {
        epd.display_fast(fb.data())?;
    } else {
        epd.display(fb.data())?;
    }
    epd.sleep()?;

    Ok(())
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let word_len = word.chars().count();
            let current_len = current.chars().count();

            if current_len == 0 && word_len > max_chars {
                for chunk in word.chars().collect::<Vec<_>>().chunks(max_chars) {
                    lines.push(chunk.iter().collect());
                }
                continue;
            }

            if current_len == 0 {
                current.push_str(word);
                continue;
            }

            if current_len + 1 + word_len <= max_chars {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current);
                current = String::new();
                if word_len > max_chars {
                    for chunk in word.chars().collect::<Vec<_>>().chunks(max_chars) {
                        lines.push(chunk.iter().collect());
                    }
                } else {
                    current.push_str(word);
                }
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}
