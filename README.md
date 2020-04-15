
# Listening example
* This example shows our TCP stack can listen for a connection from 192.168.0.1 and recieves an mp4 file.   
`bash run.sh`  
`sudo tshark -i tun0 -f "tcp"`

# Building connection actively
* This example shows that our TCP can actively connect to 192.168.0.1:port_number and tears down connection.
`bash run2.sh`  
`sudo tshark -i tun0 -f "tcp"`
