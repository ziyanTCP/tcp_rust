#!/usr/bin/env bash
export RUST_LOG=info
cargo b
# debug version or --release
ext=$?
# echo "$ext"
if [[ $ext -ne 0 ]]; then
  exit $ext
fi

sudo setcap cap_net_admin=eip target/debug/tcp_proto
#RUST_LOG=debug target/debug/tcp_proto &
target/debug/tcp_proto &
pid=$!

sudo ip addr add 192.168.0.1/24 dev tun0
sudo ip link set up dev tun0
trap "kill $pid" INT TERM # only in bash
wait $pid