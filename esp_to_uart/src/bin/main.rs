#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{
    IpAddress, IpEndpoint, IpListenEndpoint, Ipv4Address, Runner, Stack, StackResources,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::prelude::Point;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    rng::Rng,
    timer::timg::TimerGroup,
    uart::{Config as UartConfig, Uart},
};
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config as WifiConfig, Interface, WifiController, sta::StationConfig,
};
use log::{info, warn};
use static_cell::StaticCell;

use crate::ssd1306::StructSetSSD1306;

extern crate alloc;

#[path = "ssd1306.rs"]
mod ssd1306;

const UART_BAUDRATE: u32 = 9600; // Change to 9600 if using bluetooth
const UART_PARITY: esp_hal::uart::Parity = esp_hal::uart::Parity::None;
const UART_STOP_BITS: esp_hal::uart::StopBits = esp_hal::uart::StopBits::_1;
const UART_DATA_BITS: esp_hal::uart::DataBits = esp_hal::uart::DataBits::_8;

const UART_TX_PIN: u8 = 5;
const UART_RX_PIN: u8 = 6;
const UDP_PORT: u16 = 3615;
const UART_TO_UDP_PORT: u16 = 2501;
const UDP_CLIENT_LIMIT: usize = 8;
const UDP_BUFFER_SIZE: usize = 4096;
const UDP_PACKET_SLOTS: usize = 8;
const DISPLAY_FRAME_SIZE: usize = 1024;
const TRANSFER_BUFFER_SIZE: usize = 1024;

const MDNS_PORT: u16 = 5353;
const MDNS_GROUP: Ipv4Address = Ipv4Address::new(224, 0, 0, 251);
const MDNS_NAME: [&[u8]; 2] = [b"uart", b"local"];
const MDNS_BUFFER_SIZE: usize = 512;

static NETWORK_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
static MDNS_RX_BUFFER: StaticCell<[u8; MDNS_BUFFER_SIZE]> = StaticCell::new();
static MDNS_TX_BUFFER: StaticCell<[u8; MDNS_BUFFER_SIZE]> = StaticCell::new();
static MDNS_RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static MDNS_TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static MDNS_QUERY: StaticCell<[u8; MDNS_BUFFER_SIZE]> = StaticCell::new();
static MDNS_PACKET: StaticCell<[u8; MDNS_BUFFER_SIZE]> = StaticCell::new();
static UDP_RX_BUFFER: StaticCell<[u8; UDP_BUFFER_SIZE]> = StaticCell::new();
static UDP_TX_BUFFER: StaticCell<[u8; UDP_BUFFER_SIZE]> = StaticCell::new();
static UDP_TO_UART: StaticCell<[u8; TRANSFER_BUFFER_SIZE]> = StaticCell::new();
static UART_TO_UDP: StaticCell<[u8; TRANSFER_BUFFER_SIZE]> = StaticCell::new();
static UDP_RX_META: StaticCell<[PacketMetadata; UDP_PACKET_SLOTS]> = StaticCell::new();
static UDP_TX_META: StaticCell<[PacketMetadata; UDP_PACKET_SLOTS]> = StaticCell::new();

esp_bootloader_esp_idf::esp_app_desc!();

/// Returns true if the query asks for an A record of `uart.local`.
fn mdns_query_matches(packet: &[u8]) -> bool {
    if packet.len() < 12 || packet[2] & 0x80 != 0 {
        return false;
    }
    let question_count = u16::from_be_bytes([packet[4], packet[5]]);
    let mut pos = 12;
    for _ in 0..question_count {
        let mut label_index = 0;
        let mut name_matches = true;
        loop {
            let Some(&len) = packet.get(pos) else {
                return false;
            };
            pos += 1;
            if len == 0 {
                break;
            }
            if len & 0xC0 != 0 {
                return false;
            }
            let Some(label) = packet.get(pos..pos + len as usize) else {
                return false;
            };
            pos += len as usize;
            match MDNS_NAME.get(label_index) {
                Some(expected) if label.eq_ignore_ascii_case(expected) => {}
                _ => name_matches = false,
            }
            label_index += 1;
        }
        if label_index != MDNS_NAME.len() {
            name_matches = false;
        }
        let Some(tail) = packet.get(pos..pos + 4) else {
            return false;
        };
        pos += 4;
        let qtype = u16::from_be_bytes([tail[0], tail[1]]);
        let qclass = u16::from_be_bytes([tail[2], tail[3]]) & 0x7FFF;
        if name_matches && (qtype == 1 || qtype == 255) && (qclass == 1 || qclass == 255) {
            return true;
        }
    }
    false
}

/// Builds an mDNS answer for `uart.local` and returns its length.
fn mdns_build_answer(out: &mut [u8], id: u16, ip: Ipv4Address) -> usize {
    out[0..2].copy_from_slice(&id.to_be_bytes());
    out[2..4].copy_from_slice(&0x8400u16.to_be_bytes());
    out[4..6].copy_from_slice(&0u16.to_be_bytes());
    out[6..8].copy_from_slice(&1u16.to_be_bytes());
    out[8..12].copy_from_slice(&[0; 4]);
    let mut pos = 12;
    for label in MDNS_NAME {
        out[pos] = label.len() as u8;
        out[pos + 1..pos + 1 + label.len()].copy_from_slice(label);
        pos += 1 + label.len();
    }
    out[pos] = 0;
    pos += 1;
    out[pos..pos + 2].copy_from_slice(&1u16.to_be_bytes());
    out[pos + 2..pos + 4].copy_from_slice(&0x8001u16.to_be_bytes());
    out[pos + 4..pos + 8].copy_from_slice(&120u32.to_be_bytes());
    out[pos + 8..pos + 10].copy_from_slice(&4u16.to_be_bytes());
    out[pos + 10..pos + 14].copy_from_slice(&ip.octets());
    pos + 14
}

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "Embassy stores task futures in its executor task arena"
)]
async fn mdns_task(stack: Stack<'static>) -> ! {
    let rx_buffer = MDNS_RX_BUFFER.init([0; MDNS_BUFFER_SIZE]);
    let tx_buffer = MDNS_TX_BUFFER.init([0; MDNS_BUFFER_SIZE]);
    let rx_meta = MDNS_RX_META.init([PacketMetadata::EMPTY; 4]);
    let tx_meta = MDNS_TX_META.init([PacketMetadata::EMPTY; 4]);
    let query = MDNS_QUERY.init([0; MDNS_BUFFER_SIZE]);
    let packet = MDNS_PACKET.init([0; MDNS_BUFFER_SIZE]);

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta[..],
        &mut rx_buffer[..],
        &mut tx_meta[..],
        &mut tx_buffer[..],
    );
    socket
        .bind(MDNS_PORT)
        .expect("Failed to bind the mDNS socket");
    if let Err(error) = stack.join_multicast_group(MDNS_GROUP) {
        warn!("mDNS multicast join failed: {:?}", error);
    }
    info!("mDNS answering for uart.local");

    loop {
        let Ok((length, sender)) = socket.recv_from(&mut query[..]).await else {
            continue;
        };
        if !mdns_query_matches(&query[..length]) {
            continue;
        }
        let Some(config) = stack.config_v4() else {
            continue;
        };
        // Legacy unicast queries (source port != 5353) get a unicast reply.
        let (id, destination) = if sender.endpoint.port == MDNS_PORT {
            (0, IpEndpoint::new(MDNS_GROUP.into(), MDNS_PORT))
        } else {
            (u16::from_be_bytes([query[0], query[1]]), sender.endpoint)
        };
        let answer_length = mdns_build_answer(packet, id, config.address.address());
        if let Err(error) = socket.send_to(&packet[..answer_length], destination).await {
            warn!("mDNS send failed: {:?}", error);
        }
    }
}

#[embassy_executor::task]
async fn network_task(mut runner: Runner<'static, Interface>) -> ! {
    runner.run().await
}

async fn write_all(uart: &mut Uart<'static, esp_hal::Async>, data: &[u8]) {
    let mut offset = 0;
    while offset < data.len() {
        match uart.write_async(&data[offset..]).await {
            Ok(written) => offset += written,
            Err(error) => {
                warn!("UART write failed: {:?}", error);
                break;
            }
        }
    }
}

fn remember_udp_client(clients: &mut [Option<IpAddress>], address: IpAddress) -> bool {
    if clients.contains(&Some(address)) {
        return true;
    }

    let Some(slot) = clients.iter_mut().find(|client| client.is_none()) else {
        return false;
    };
    *slot = Some(address);
    true
}

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "Embassy stores task futures in its executor task arena"
)]
async fn bridge_task(
    stack: Stack<'static>,
    mut uart: Uart<'static, esp_hal::Async>,
    mut display: Option<StructSetSSD1306>,
) -> ! {
    let rx_buffer = UDP_RX_BUFFER.init([0; UDP_BUFFER_SIZE]);
    let tx_buffer = UDP_TX_BUFFER.init([0; UDP_BUFFER_SIZE]);
    let rx_meta = UDP_RX_META.init([PacketMetadata::EMPTY; UDP_PACKET_SLOTS]);
    let tx_meta = UDP_TX_META.init([PacketMetadata::EMPTY; UDP_PACKET_SLOTS]);

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta[..],
        &mut rx_buffer[..],
        &mut tx_meta[..],
        &mut tx_buffer[..],
    );
    // Port only, no address: accepts datagrams sent to any local address.
    socket
        .bind(IpListenEndpoint {
            addr: None,
            port: UDP_PORT,
        })
        .expect("Failed to bind the UDP socket");
    info!("Listening for UDP datagrams on 0.0.0.0:{}", UDP_PORT);

    let udp_to_uart = UDP_TO_UART.init([0; TRANSFER_BUFFER_SIZE]);
    let uart_to_udp = UART_TO_UDP.init([0; TRANSFER_BUFFER_SIZE]);
    let mut udp_clients = [None; UDP_CLIENT_LIMIT];
    let mut client_limit_warned = false;

    loop {
        match select(
            socket.recv_from(&mut udp_to_uart[..]),
            uart.read_async(&mut uart_to_udp[..]),
        )
        .await
        {
            Either::First(Ok((length, sender))) => {
                if !remember_udp_client(&mut udp_clients, sender.endpoint.addr)
                    && !client_limit_warned
                {
                    warn!(
                        "UDP client limit ({}) reached; additional client IPs will not receive UART replies",
                        UDP_CLIENT_LIMIT
                    );
                    client_limit_warned = true;
                }

                if length == DISPLAY_FRAME_SIZE {
                    let frame: &[u8; DISPLAY_FRAME_SIZE] = udp_to_uart[..DISPLAY_FRAME_SIZE]
                        .try_into()
                        .expect("slice length equals DISPLAY_FRAME_SIZE");
                    if let Some(display) = display.as_mut()
                        && let Err(error) =
                            display.set_from_1024_bytes_array_top_left_to_down_right(frame)
                    {
                        warn!("Display update failed: {:?}", error);
                    }
                } else {
                    write_all(&mut uart, &udp_to_uart[..length]).await;
                }
            }
            Either::First(Err(error)) => {
                warn!("UDP receive failed: {:?}", error);
            }
            Either::Second(Ok(length)) => {
                for address in udp_clients.iter().flatten() {
                    let destination = IpEndpoint::new(*address, UART_TO_UDP_PORT);
                    if let Err(error) = socket.send_to(&uart_to_udp[..length], destination).await {
                        warn!("UART-to-UDP send failed: {:?}", error);
                    }
                }
            }
            Either::Second(Err(error)) => {
                warn!("UART read failed: {:?}", error);
            }
        }
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "the ESP32 entry point owns peripherals and initializes larger buffers"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let wifi_ssid_default = "UART.LOCAL";
    let wifi_password_default = "12345678";

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let _gpio27 = peripherals.GPIO27;
    let _gpio28 = peripherals.GPIO28;
    let _gpio29 = peripherals.GPIO29;
    let _gpio30 = peripherals.GPIO30;
    let _gpio31 = peripherals.GPIO31;
    let _gpio32 = peripherals.GPIO32;
    let mut led = Output::new(peripherals.GPIO48, Level::Low, OutputConfig::default());

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    info!("Embassy initialized");

    let i2c =
        esp_hal::i2c::master::I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
            .unwrap()
            .with_sda(peripherals.GPIO7)
            .with_scl(peripherals.GPIO8);
    let mut display = match StructSetSSD1306::new(i2c) {
        Ok(display) => Some(display),
        Err(error) => {
            warn!(
                "SSD1306 not found at 0x3C/0x3D on SDA GPIO{} / SCL GPIO{}: {:?}; continuing without display",
                ssd1306::SSD1306_SDA_PIN,
                ssd1306::SSD1306_SCL_PIN,
                error
            );
            None
        }
    };
    if let Some(display) = display.as_mut() {
        let _ = display.set_black();
        let _ = display.set_text("Wi-Fi UART\nConnecting...", Point::new(0, 0));
    }
    info!(
        "OLED I2C uses SDA GPIO{} and SCL GPIO{}",
        ssd1306::SSD1306_SDA_PIN,
        ssd1306::SSD1306_SCL_PIN
    );

    let mut wifi_controller =
        WifiController::new(peripherals.WIFI, Default::default()).expect("Wi-Fi init failed");
    let wifi_interface = Interface::station();
    let station_config = StationConfig::default()
        .with_ssid(
            wifi_ssid_default
                .try_into()
                .expect("ESP_WIFI_SSID is longer than the supported Wi-Fi SSID length"),
        )
        .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
            wifi_password_default
                .try_into()
                .expect("ESP_WIFI_PASSWORD is longer than the supported password length"),
        ));
    wifi_controller
        .set_config(&WifiConfig::Station(station_config))
        .expect("Wi-Fi configuration failed");

    let network_resources = NETWORK_RESOURCES.init(StackResources::new());
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        embassy_net::Config::dhcpv4(Default::default()),
        network_resources,
        Rng::new().random() as u64,
    );
    spawner.spawn(network_task(runner).expect("Network task storage is exhausted"));

    wifi_controller
        .connect_async()
        .await
        .expect("Failed to connect to the Wi-Fi network");
    info!("Connected to Wi-Fi; waiting for a DHCP address");
    stack.wait_config_up().await;
    info!("Network ready: {:?}", stack.config_v4());
    if let Some(display) = display.as_mut() {
        let _ = display.set_black();
    }

    let uart_config = UartConfig::default()
        .with_baudrate(UART_BAUDRATE)
        .with_parity(UART_PARITY)
        .with_stop_bits(UART_STOP_BITS)
        .with_data_bits(UART_DATA_BITS);
    let uart = Uart::new(peripherals.UART1, uart_config)
        .expect("UART1 configuration failed")
        .with_rx(peripherals.GPIO6)
        .with_tx(peripherals.GPIO5)
        .into_async();

    info!(
        "UART1 bidirectional: TX GPIO{}, RX GPIO{} at {} baud",
        UART_TX_PIN, UART_RX_PIN, UART_BAUDRATE
    );
    let ipv4 = stack.config_v4().unwrap().address;

    if let Some(display) = display.as_mut() {
        let _ = display.set_text(
            &alloc::format!(
                "Wi-Fi connected\nUDP in: {}\nUART 5/6: {} baud\nReply port: {}\nIP: {}",
                UDP_PORT,
                UART_BAUDRATE,
                UART_TO_UDP_PORT,
                ipv4
            ),
            Point::new(0, 0),
        );
    }

    spawner.spawn(bridge_task(stack, uart, display).expect("Bridge task storage is exhausted"));
    spawner.spawn(mdns_task(stack).expect("mDNS task storage is exhausted"));

    loop {
        Timer::after(Duration::from_millis(500)).await;
        led.toggle();
    }
}
