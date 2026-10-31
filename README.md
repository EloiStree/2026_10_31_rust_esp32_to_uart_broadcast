# 2026_10_31_rust_esp32_to_uart
I need to add WiFi to Micro:Bit and Arduino with level shifter, so I want to check if I can Rust my way to that with Esp32


Install on window
https://rustup.rs/
Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`


Powershell
```
cargo install espup --locked
cargo install espflash --locked
cargo install esp-generate --locked
espup install
. "$HOME\export-esp.ps1"
cargo install esp-config --locked
irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex

rustc --version
cargo --version
espflash --version
xtensa-esp32s3-elf-gcc --version
esp-config --version
probe-rs --version
```

To buid and run:
```
cargo run --release
```


