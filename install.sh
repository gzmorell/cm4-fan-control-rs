#!/bin/bash

service_exists() {
  local n
  n=$1
  if [[ $(systemctl list-units --all -t service --full --no-legend "$n.service" | sed 's/^\s*//g' | cut -f1 -d' ') == $n.service ]]; then
      return 0
  else
      return 1
    fi
}

cargo build --release

if service_exists cm4_fan_control; then
  sudo systemctl stop cm4_fan_control
fi
# Install the package
sudo cp target/aarch64-unknown-linux-gnu/release/cm4_fan_control /usr/local/sbin/
sudo cp lib/cm4_fan_control.service /etc/systemd/system/cm4_fan_control.service

if service_exists cm4_fan_control; then
  sudo systemctl daemon-reload
  sudo systemctl restart cm4_fan_control
else
  sudo systemctl enable cm4_fan_control.service
  sudo systemctl start cm4_fan_control.service
fi
