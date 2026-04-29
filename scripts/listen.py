import socket

# The port defined in net_ota.cpp
UDP_PORT = 3232

def listen_for_broadcast():
    # Create a UDP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    
    # Allow the socket to reuse the address (helps if the script is restarted quickly)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # Bind to all interfaces on the specific port
    try:
        sock.bind(('', UDP_PORT))
    except PermissionError:
        print(f"Error: Permission denied binding to port {UDP_PORT}.")
        return
    except Exception as e:
        print(f"Error binding to port: {e}")
        return

    print(f"Listening for Meshtastic OTA broadcasts on UDP port {UDP_PORT}...")
    print("Press Ctrl+C to stop.")

    try:
        while True:
            # Receive data (buffer size 1024 bytes)
            data, addr = sock.recvfrom(1024)
            
            try:
                message = data.decode('utf-8')
            except UnicodeDecodeError:
                message = data.hex()

            ip_address = addr[0]
            
            print("-" * 40)
            print(f"Device Found!")
            print(f"IP Address : {ip_address}")
            print(f"Message    : {message}")
            print(f"Action     : You can now connect via TCP to {ip_address}:{UDP_PORT}")
            
    except KeyboardInterrupt:
        print("\nStopping listener...")
    finally:
        sock.close()

if __name__ == "__main__":
    listen_for_broadcast()
