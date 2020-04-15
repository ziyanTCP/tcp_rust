
# Listening example
* This example shows our TCP listening for a connection from 192.168.0.1 and receiving an mp4 file.   
`bash run.sh`  
`sudo tshark -i tun0 -f "tcp"`

# Building connection actively
* this example shows out TCP actively connecting to 192.168.0.1:port_number and tearing down connection  

`bash run2.sh`  
`sudo tshark -i tun0 -f "tcp"`