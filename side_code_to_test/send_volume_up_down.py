import select
import socket
import struct
import time


UDP_IP = "192.168.137.50"
UDP_PORT = 3615
UDP_REPLY_PORT = 2501


# Try to fetch IPv4 address of the "uart.local" mDNS hostname
try:
    resolved_ip = socket.gethostbyname("uart.local")
    print(f"Resolved uart.local -> {resolved_ip}")
    UDP_IP = resolved_ip
except socket.gaierror:
    print(f"Could not resolve uart.local, using default IP {UDP_IP}")


sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(("", UDP_REPLY_PORT))

values = [1175, 1174,1300,1301,1302,1303,1304,1305,2300,2301,2302,2303,2304,2305]

values = [1175, 1174,1800,1801,1802,1803,1804,1805,2800,2801,2802,2803,2804,2805, 1841, 1849, 1851, 1859, 1861, 1869, 1871, 1879, 1881, 1889, 1891, 1899,"LED_ON" ,"LED_OFF"]


def send_value(value):
    data = f"{value}\n".encode("utf-8")
    sock.sendto(data, (UDP_IP, UDP_PORT))
    print(f"Sent {value} to {UDP_IP}:{UDP_PORT} -> {value}")
    readable, _, _ = select.select([sock], [], [], 1)
    while readable:
        data, sender = sock.recvfrom(4096)
        print(f"UART reply from {sender}: {data!r}")
        readable, _, _ = select.select([sock], [], [], 0)

if __name__ == "__main__":
    try:
        while True:
            for value in values:
                send_value(value)
                time.sleep(1)
    except KeyboardInterrupt:
        print("Stopped by user")
    finally:
        sock.close()
