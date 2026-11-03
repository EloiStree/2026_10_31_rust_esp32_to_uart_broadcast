import secrets
import socket
from time import sleep, time

PORT = 3615

UDP_IP = "192.168.137.50"
# Try to fetch IPv4 address of the "uart.local" mDNS hostname
try:
    resolved_ip = socket.gethostbyname("uart.local")
    print(f"Resolved uart.local -> {resolved_ip}")
    UDP_IP = resolved_ip
except socket.gaierror:
    print(f"Could not resolve uart.local, using default IP {UDP_IP}")

payload = secrets.token_bytes(1024)
print(f"Sending payload to {UDP_IP}:{PORT}")


WHITE_SCREEN = secrets.token_bytes(1024)
BLACK_SCREEN = secrets.token_bytes(1024)


line_index = 0
while True:
    sleep(1.0/60.0)
    # with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as connection:
    #     #Randomise the payload for each send
    #     payload = secrets.token_bytes(1024)
    #     print(f"Sending payload to {UDP_IP}:{PORT}")
    #     connection.sendto(payload, (UDP_IP, PORT))
    

    line_index = (line_index + 1) % 8
    payload = bytearray(secrets.token_bytes(1024))
    random_value = secrets.randbits(8)
    for i in range(128):
        payload[line_index * 128 + i] = random_value
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as connection:
        connection.sendto(payload, (UDP_IP, PORT))

