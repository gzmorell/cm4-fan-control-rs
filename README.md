# Raspberry Pi CM4 IO Fan Control

A simple utility written in Rust to help control the fan speed of a Raspberry Pi CM4 IO Board using the i2c
fan control hardware to avoid the to high CPU usage.

In order to activate the fan control, I2C bus #1 and #0 must be enabled. The [PI Device Tree Documentation](https://www.raspberrypi.com/documentation/computers/configuration.html#part3.3) recomends not using this changes, as it can 
stop the Raspberry Pi Camera or Raspberry Pi Touch Display functioning correctly. 

To activate i2c for fan "config.txt" on boot must include:
# Enable I2C.
dtparam=i2c_arm=on
# Enable I2C bus 0/1.
dtparam=i2c_vc=on

It is based on the app [rpi-fan-control-rs](https://github.com/mihirsamdarshi/rpi-fan-control-rs.git) which uses
a more general approach, and do not use i2c.

It uses the same simple fan curve as the original app, seen below.

![Graph of the fan curve](img/curve.png)

## Build

The expected target is aarch64-unknown-linux-gnu.

To cross compile you can use cross, cargo-zigbuild, or setup a debian cross compilation:
''' bash
> sudo dpkg --add-architecture arm64
> sudo apt-get install pkg-config build-essential crossbuild-essential-arm64
> cargo deb --target="aarch64-unknown-linux-build" 
'''

To create a debian package after cross compilation you can use cargo-deb with the next command:
''' bash
> cargo deb --target="aarch64-unknown-linux-build" --no-build
'''

To compile and build the debian package on the cm4:
''' bash
> cargo deb --target="aarch64-unknown-linux-build" 
'''
