use clap::{Parser, Subcommand};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use rpi_einkserver_rs::{Epd2in13V4, EpdPins, MonoImage};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "rpi-einkserver-rs",
    author,
    version,
    about = "Minimal demo for the Waveshare 2.13\" V4 e-Paper HAT"
)]
struct Cli {
    /// Use the fast init/display path.
    #[arg(long, short)]
    fast: bool,

    /// Skip initialization (assumes panel is already configured).
    #[arg(long)]
    noinit: bool,

    /// Swap foreground/background colors.
    #[arg(long)]
    reverse_color: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Initialize and clear the display before sleeping.
    Clear,
    /// Initialize and write a wrapped message to the display.
    Write {
        /// Text to render (wrapped to fit the display).
        #[arg(long)]
        text: Option<String>,
    },
    /// Interactive stdin REPL for issuing commands or text.
    Repl,
    /// Serve REPL-like commands over a Unix socket for scripting.
    Serve {
        /// Path to the Unix socket to bind, e.g. /tmp/eink.sock.
        #[arg(long, short = 's', default_value = "/tmp/eink.sock")]
        socket: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Default Waveshare HAT pins (BCM numbering): BUSY=24, RST=17, DC=25, CS=8.
    let pins = EpdPins {
        busy: 24,
        dc: 25,
        cs: 8,
        rst: 17,
    };

    let mut epd = Epd2in13V4::new(pins)?;

    let fg_color = if cli.reverse_color {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let bg_color = if cli.reverse_color {
        BinaryColor::On
    } else {
        BinaryColor::Off
    };

    let command = cli
        .command
        .clone()
        .unwrap_or(Command::Write { text: None });

    match command {
        Command::Clear => {
            maybe_init(&mut epd, &cli)?;
            epd.clear(bg_color)?;
            epd.sleep()?;
        }
        Command::Write { text } => {
            maybe_init(&mut epd, &cli)?;
            let message = text
                .map(|t| decode_newlines(&t))
                .unwrap_or_else(|| {
                    "Hello from Rust! Pass --write --text \"your message\" to set custom text."
                        .to_string()
                });
            render_text(&mut epd, &message, fg_color, bg_color, cli.fast)?;
            epd.sleep()?;
        }
        Command::Repl => run_repl(epd, &cli, fg_color, bg_color)?,
        Command::Serve { socket } => run_server(epd, &cli, fg_color, bg_color, &socket)?,
    }

    Ok(())
}

fn maybe_init(epd: &mut Epd2in13V4, cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if cli.noinit {
        println!("Skipping panel initialization as requested.");
        return Ok(());
    }

    if cli.fast {
        epd.init_fast()?;
    } else {
        epd.init()?;
    }
    Ok(())
}

fn render_text(
    epd: &mut Epd2in13V4,
    message: &str,
    fg: BinaryColor,
    bg: BinaryColor,
    fast: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let fb = build_framebuffer(message, fg, bg);
    if fast {
        epd.display_fast(fb.data())?;
    } else {
        epd.display(fb.data())?;
    }
    Ok(())
}

fn build_framebuffer(message: &str, fg: BinaryColor, bg: BinaryColor) -> MonoImage {
    let mut fb = MonoImage::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32);
    fb.clear(bg);

    Rectangle::new(
        Point::new(0, 0),
        Size::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(fg, 1))
    .draw(&mut fb)
    .ok();

    let margin = 6i32;
    let char_width = 10usize;
    let line_height = 20i32;
    let max_chars = ((Epd2in13V4::WIDTH as usize).saturating_sub((margin as usize) * 2)
        / char_width)
        .max(1);
    let max_lines = (Epd2in13V4::HEIGHT as usize).saturating_sub((margin as usize) * 2)
        / line_height as usize;
    let lines = wrap_text(message, max_chars);

    let style = MonoTextStyle::new(&FONT_10X20, fg);
    let mut y = margin + 20;
    for line in lines.into_iter().take(max_lines) {
        Text::new(&line, Point::new(margin, y), style)
            .draw(&mut fb)
            .ok();
        y += line_height;
    }

    fb
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

fn blank_framebuffer(bg: BinaryColor) -> MonoImage {
    let mut fb = MonoImage::new(Epd2in13V4::WIDTH as u32, Epd2in13V4::HEIGHT as u32);
    fb.clear(bg);
    fb
}

fn run_repl(
    mut epd: Epd2in13V4,
    cli: &Cli,
    fg: BinaryColor,
    bg: BinaryColor,
) -> Result<(), Box<dyn std::error::Error>> {
    maybe_init(&mut epd, cli)?;

    println!(
        "REPL ready. Commands: /clear, /partial, /nopartial. Type text to display. Ctrl-D to exit."
    );

    let stdin = io::stdin();
    let mut partial = false;

    for line in stdin.lock().lines() {
        let line = line?;

        if line.starts_with('/') {
            match line.as_str() {
                "/clear" => {
                    epd.clear(bg)?;
                }
                "/partial" => {
                    let blank = blank_framebuffer(bg);
                    epd.display_base(blank.data())?;
                    partial = true;
                    println!("Partial updates enabled.");
                }
                "/nopartial" => {
                    partial = false;
                    println!("Partial updates disabled.");
                }
                other => {
                    println!("Unknown command: {other}");
                }
            }
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }

        let text = decode_newlines(&line);
        let fb = build_framebuffer(&text, fg, bg);
        if partial {
            epd.display_partial(fb.data())?;
        } else if cli.fast {
            epd.display_fast(fb.data())?;
        } else {
            epd.display(fb.data())?;
        }
    }

    epd.sleep()?;
    Ok(())
}

fn decode_newlines(input: &str) -> String {
    input.replace("\\n", "\n")
}

fn run_server(
    mut epd: Epd2in13V4,
    cli: &Cli,
    fg: BinaryColor,
    bg: BinaryColor,
    socket: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if socket.exists() {
        std::fs::remove_file(socket)?;
    }

    maybe_init(&mut epd, cli)?;

    let listener = UnixListener::bind(socket)?;
    println!(
        "Unix socket server listening on {}",
        socket.to_string_lossy()
    );
    println!("Protocol: newline-delimited packets. Commands: TEXT <msg> (default), CLEAR, PARTIAL_ON, PARTIAL_OFF, PING.");

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                if let Err(err) = handle_connection(stream, &mut epd, cli, fg, bg) {
                    eprintln!("Connection error: {err}");
                }
            }
            Err(err) => eprintln!("Accept error: {err}"),
        }
    }

    Ok(())
}

fn handle_connection(
    stream: UnixStream,
    epd: &mut Epd2in13V4,
    cli: &Cli,
    fg: BinaryColor,
    bg: BinaryColor,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = stream;
    let reader_stream = writer.try_clone()?;
    let mut reader = BufReader::new(reader_stream);

    let mut line = String::new();
    let mut partial = false;

    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }

        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.is_empty() {
            continue;
        }

        let (cmd, payload) = parse_packet(trimmed);
        let response = match cmd {
            PacketCommand::Clear => {
                epd.clear(bg)?;
                "OK CLEAR"
            }
            PacketCommand::PartialOn => {
                let blank = blank_framebuffer(bg);
                epd.display_base(blank.data())?;
                partial = true;
                "OK PARTIAL_ON"
            }
            PacketCommand::PartialOff => {
                partial = false;
                "OK PARTIAL_OFF"
            }
            PacketCommand::Ping => "PONG",
            PacketCommand::Text => {
                let text = decode_newlines(payload.unwrap_or_default());
                if text.trim().is_empty() {
                    "IGNORED EMPTY"
                } else {
                    let fb = build_framebuffer(&text, fg, bg);
                    if partial {
                        epd.display_partial(fb.data())?;
                    } else if cli.fast {
                        epd.display_fast(fb.data())?;
                    } else {
                        epd.display(fb.data())?;
                    }
                    "OK TEXT"
                }
            }
        };

        respond(&mut writer, response)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PacketCommand {
    Text,
    Clear,
    PartialOn,
    PartialOff,
    Ping,
}

fn parse_packet(input: &str) -> (PacketCommand, Option<&str>) {
    let mut parts = input.splitn(2, char::is_whitespace);
    let head = parts.next().unwrap_or("");
    let payload = parts.next();

    match head.to_ascii_uppercase().as_str() {
        "CLEAR" => (PacketCommand::Clear, None),
        "PARTIAL_ON" => (PacketCommand::PartialOn, None),
        "PARTIAL_OFF" => (PacketCommand::PartialOff, None),
        "PING" => (PacketCommand::Ping, None),
        "TEXT" => (PacketCommand::Text, payload),
        _ => (PacketCommand::Text, Some(input)),
    }
}

fn respond(stream: &mut UnixStream, message: &str) -> io::Result<()> {
    stream.write_all(message.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}
