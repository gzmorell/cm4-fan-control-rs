# Raspberry Pi CM4 IO Fan Control

A simple utility written in Rust to help control the fan speed of a Raspberry Pi CM4 IO Board using the i2c
fan control hardware to avoid the to high CPU usage

It is based on the app [rpi-fan-control-rs](https://github.com/mihirsamdarshi/rpi-fan-control-rs.git)

It uses the same simple fan curve as the original app, seen below.

![Graph of the fan curve](img/curve.png)
