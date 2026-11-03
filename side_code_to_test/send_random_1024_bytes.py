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

while True:
    sleep(0.4)
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as connection:
        #Randomise the payload for each send
        payload = secrets.token_bytes(1024)
        print(f"Sending payload to {UDP_IP}:{PORT}")
        connection.sendto(payload, (UDP_IP, PORT))
