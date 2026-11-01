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

To build and run, first set the Wi-Fi environment variables described below:
```
cd esp_to_uart
cargo run --release
```

## Wi-Fi UDP to UART bridge

The firmware joins a WPA2 Wi-Fi network and listens for UDP datagrams on
`0.0.0.0:3615`. There is no reply traffic: nothing is sent back over UDP.

- A datagram of exactly **1024 bytes** is drawn on the SSD1306 OLED
  (128x64, one byte per 8 vertical pixels, page by page from top-left to
  bottom-right, LSB at the top).
- Any other datagram has its payload written to UART1 as raw bytes, without
  framing or line endings.

Set the Wi-Fi credentials in the PowerShell environment before building. They
are embedded in the firmware at compile time, so do not commit them:

```powershell
$env:ESP_WIFI_SSID = "your-wifi-name"
$env:ESP_WIFI_PASSWORD = "your-wifi-password"
Set-Location .\esp_to_uart
cargo run --release
```


