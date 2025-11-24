# Waveshare 2.13" e‑Paper HAT (V4) in Rust

Rust driver and example for the Waveshare 2.13" e‑Paper HAT (V4) on Raspberry Pi. It speaks to the panel over SPI using `rppal` and exposes a simple framebuffer that works with `embedded-graphics`.

## Hardware

- Panel: Waveshare 2.13" e‑Paper HAT (V4)
- SPI bus: `/dev/spidev0.0` (default Raspberry Pi pins)
- Control pins (BCM): `BUSY=24`, `RST=17`, `DC=25` (adjust in `src/main.rs` or when constructing `EpdPins`)
- Enable SPI in `raspi-config` and ensure the running user is in the `spi` group or run with sudo.

## Build and run

```bash
sudo apt-get install pkg-config libssl-dev # typical Rust toolchain deps on Pi
cargo run --release
```

The demo draws a border, a couple of shapes, and text, then sends the panel to sleep.

## CLI

- `cargo run --release -- write --text "Hello"`: init (unless `--noinit`), render text, sleep. Add `--fast` for the fast LUT.
- `cargo run --release -- clear`: init (unless `--noinit`), clear, sleep.
- `cargo run --release -- repl`: interactive stdin REPL. Commands: `/clear`, `/partial`, `/nopartial`; other lines are rendered. Exits on EOF.
- `cargo run --release -- serve --socket /tmp/eink.sock`: REPL-like service over a Unix socket (default `/tmp/eink.sock`). Removes any stale socket before binding.

### Socket protocol (newline-delimited)

- `TEXT <msg>` or a bare line: render text (supports `\n` escapes). Replies `OK TEXT` or `IGNORED EMPTY`.
- `CLEAR`: clears the display. Reply: `OK CLEAR`.
- `PARTIAL_ON`: send a blank base frame then enable partial updates for subsequent text. Reply: `OK PARTIAL_ON`.
- `PARTIAL_OFF`: return to full updates. Reply: `OK PARTIAL_OFF`.
- `PING`: health check. Reply: `PONG`.

Example client:

```bash
printf 'TEXT hello\\nPING\\n' | socat - UNIX-CONNECT:/tmp/eink.sock
```

## Library overview

- `Epd2in13V4`: driver with `init`, `display`, `display_fast`, `display_base`, `display_partial`, `clear`, and `sleep`.
- `MonoImage`: 1‑bit framebuffer implementing `embedded-graphics` `DrawTarget`. Call `data()` to get the byte slice to hand to the driver.
- Update modes map to the Waveshare reference values (`UpdateMode::Normal/Fast/Partial`).

## Notes

- Width/height: 122x250 px; bytes per row = 16.
- The refresh sequences mirror the official C driver (see `vendor-context/e-Paper/.../EPD_2in13_V4.c`), but only the V4 panel is wired up here.
- Partial refresh is provided but untested without hardware; prefer full updates until validated.
