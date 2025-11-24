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

## Library overview

- `Epd2in13V4`: driver with `init`, `display`, `display_fast`, `display_base`, `display_partial`, `clear`, and `sleep`.
- `MonoImage`: 1‑bit framebuffer implementing `embedded-graphics` `DrawTarget`. Call `data()` to get the byte slice to hand to the driver.
- Update modes map to the Waveshare reference values (`UpdateMode::Normal/Fast/Partial`).

## Notes

- Width/height: 122x250 px; bytes per row = 16.
- The refresh sequences mirror the official C driver (see `vendor-context/e-Paper/.../EPD_2in13_V4.c`), but only the V4 panel is wired up here.
- Partial refresh is provided but untested without hardware; prefer full updates until validated.
