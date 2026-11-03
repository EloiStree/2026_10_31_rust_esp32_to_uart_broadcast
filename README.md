# 2026_10_31_rust_esp32_to_uart


I need to add WiFi to Micro:Bit and Arduino with level shifter, so I want to check if I can Rust my way to that with Esp32

<img width="1371" height="940" alt="image" src="https://github.com/user-attachments/assets/eb7fa4f3-8fea-4271-ae8f-4e312d5f2a5d" />


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



[<img width="1038" height="618" alt="image" src="https://github.com/user-attachments/assets/4e0ad815-03e4-425b-8b91-de0bbdde3693" />](https://docs.freenove.com/projects/fnk0101/en/latest/fnk0101/codes/tutorial/2_ESP32.html)   
https://docs.freenove.com/projects/fnk0101/en/latest/fnk0101/codes/tutorial/2_ESP32.html   


<img width="1361" height="1019" alt="image" src="https://github.com/user-attachments/assets/05e07e8d-dc66-477b-a48f-76a114620d22" />




Hardware used with this tool:
- Seeed Studio XIAO ESP32-S3
- SSD1306
- Level shifter for UART connections to 5 V devices
- Arduino Leonardo Keyestudio
- Freenove Breakout Board


Hardware that would be nice for this tool, if it works:
- [Pro Micro Controller 32U4](https://www.amazon.com.be/-/en/Controller-Update-Interface-Compatible-Arduino/dp/B0D9D5KW2Q/ref=asc_df_B0D9D5KW2Q) instead of Leonardo

## XIAO ESP32-S3 wiring and data flow

The firmware uses one full-duplex UART at 9600 baud, 8-N-1:

- XIAO GPIO5 (D4, TX) -> other device RX
- XIAO GPIO6 (D5, RX) <- other device TX
- Connect the device grounds together.
- SSD1306 I2C: SDA GPIO7 (D8), SCL GPIO8 (D9), address `0x3C` or `0x3D`.

GPIOs on the XIAO are 3.3 V. Never connect a 5 V UART TX signal directly to GPIO6; use a suitable level shifter. Translate GPIO5's signal too if the receiving device requires 5 V logic.

UDP clients send datagrams to the device on port `3615`; the firmware writes them to UART TX. A 1024-byte datagram is reserved for an SSD1306 frame and is displayed instead of being sent over UART. UART RX data is sent back to each remembered client IP on UDP port `2501`. A client must first send a datagram to port `3615`; up to 8 distinct IPs are remembered until reboot. Bind the client's UDP listener to port `2501` to receive UART data. UART data is forwarded as read chunks, not as line-delimited messages.


Level Shifter to explore in the future:
- PC817	Relatively slow
- BSS138 module	Not ideal for fast push-pull UART
- TXS0108E	Up to 110 Mbps under specified conditions
- 74AHCT125	Very fast, simple one-way buffer    
- SN74LVC1T45	Fast, dual-supply translator


Maybe use a one to many 5v and 3.3v 74AHCT125 / 74LVC125A to server several device (as I only push data)







# Simulate Key with UART

## XInput Arduino : CPP

<img width="831" height="480" alt="image" src="https://github.com/user-attachments/assets/ff8e0278-90b1-476e-b63b-485624f4d63d" />

https://github.com/EloiStree/2023_06_21_arduino_to_xbox_360

## Raspberry Pico : Python 

<img width="555" height="737" alt="image" src="https://github.com/user-attachments/assets/eea0ef00-695a-4ec5-b45d-73fe47f1deca" />

https://github.com/EloiStree/picoS2W

## Micro:Bit : MakerCode

<img width="866" height="485" alt="image" src="https://github.com/user-attachments/assets/15bd4728-5c55-4fac-bc3e-0bcf160f8e9c" />

[<img width="1649" height="924" alt="image" src="https://github.com/user-attachments/assets/c7641647-6044-4526-be98-938d46f2e2dc" />](https://github.com/bsiever/microbit-pxt-blehid)   
https://github.com/bsiever/microbit-pxt-blehid  

Example:    
- https://github.com/eloistree/2026_02_11_makecode_hello_uart_in        
- https://github.com/EloiStree/2026_09_12_makecode_hello_uart_bluetooth_ks_4036
- https://github.com/EloiStree/2026_09_12_makecode_hello_uart_bluetooth_ks_4036/tree/pins_ble_input_version
