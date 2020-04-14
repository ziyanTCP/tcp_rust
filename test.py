import socket
import sys

# Create a TCP/IP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

def get_bytes_from_file(filename):
    return open(filename, "rb").read()

result=get_bytes_from_file("test.mp4")

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server_address = ('192.168.0.2', 4000)

sock.connect(server_address)

sock.send(result)

sock.close()