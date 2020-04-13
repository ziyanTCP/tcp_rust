
To see a simple example of how the TCP works  
run the following command to start our tcp process
`bash run.sh`

start tshark to capture the packets
`sudo tshark -i tun0 -f "tcp" `

start the tcp clients provided by Python
`python test.py`
