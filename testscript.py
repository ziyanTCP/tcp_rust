import socket
import sys
# Create a TCP/IP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server_address = ('192.168.0.2', 4000)
sock.connect(server_address)
sock.send(b'hello')
sock.send(b"world")
sock.close()