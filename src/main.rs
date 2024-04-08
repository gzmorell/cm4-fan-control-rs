use std::{
    f32::consts::PI,
    io::ErrorKind,
    sync::{Arc, Mutex},
    time::Duration,
};

use once_cell::sync::Lazy;
use rppal::i2c::I2c;

// I2C data
/// The bus number for fan control
const I2C_BUS: u8 = 10;
/// The slave address for fan control
const I2C_SLA: u16 = 0x2f;
/// The SMBUS command for writing or reading fan speed
const I2C_CMD: u8 = 0x30;

/// The PWM frequency that the PWM fan should operate at (for the Noctua A4x10)
// const PWM_FREQUENCY: f64 = 25_000.0;
/// [°C] temperature below which to stop the fan
const OFF_TEMP: f32 = 40.0;
/// [°C] temperature above which to start the fan
const MIN_TEMP: f32 = 45.0;
/// [°C] temperature above which to start the fan at full speed
const MAX_TEMP: f32 = 75.0;

/// The speed (percentage) that the fan is off at
const FAN_OFF: f32 = 0.0;
/// The speed (percentage) that the lowest setting of the fan should be
const FAN_LOW: f32 = 0.1;
/// The speed (percentage) that the max setting of the fan is
const FAN_MAX: f32 = 1.0;
/// The steps that the fan speed should increase per each degree that the
/// temperature increase
const FAN_GAIN: f32 = (FAN_MAX - FAN_LOW) / (MAX_TEMP - MIN_TEMP);
/// The max speed setting for the fan
const MAX_SPEED: u8 = 0xFF;

static FAN_SPEED: Lazy<Arc<Mutex<u8>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

const UDEV_ERROR: &str = r#"
Access to the i2c bus should not require root permission, but the user should be in the i2c group.
"#;

// const PWM_PERMISSION_ERROR: &str = r#"
// By default, both channels are disabled.

// To enable only PWM0 on its default pin (BCM GPIO 18, physical pin 12), add dtoverlay=pwm to /boot/config.txt on Raspberry Pi OS or boot/firmware/usercfg.txt on Ubuntu.
// If you need both PWM channels, replace pwm with pwm-2chan, which enables PWM0 on BCM GPIO 18 (physical pin 12), and PWM1 on BCM GPIO 19 (physical pin 35).
// More details on enabling and configuring PWM on other GPIO pins than the default ones can be found in /boot/overlays/README.
// "#;

// const GPIO_PERMISSION_ERROR: &str = r#"
// In recent releases of Raspberry Pi OS (December 2017 or later), users that are part of the gpio group (like the default pi user) can access /dev/gpiomem and /dev/gpiochipN (N = 0-2) without needing additional permissions.
// Either the current user isn’t a member of the gpio group, or your Raspberry Pi OS distribution isn't up-to-date and doesn't automatically configure permissions for the above-mentioned files.
// Updating Raspberry Pi OS to the latest release should fix any permission issues.
// Alternatively, although not recommended, you can run your application with superuser privileges by using sudo.

// If you’re unable to update Raspberry Pi OS and its packages (namely raspberrypi-sys-mods) to the latest available release, or updating hasn't fixed the issue, you might be able to manually update your udev rules to set the appropriate permissions.
// More information can be found at https://github.com/raspberrypi/linux/issues/1225 and https://github.com/raspberrypi/linux/issues/2289.
// "#;

/// Returns the temperature of the CPU in degrees Celsius.
fn get_cpu_temp() -> f32 {
    let temp_unparsed = match std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        Ok(temp) => temp,
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                panic!("Failed to read /sys/class/thermal/thermal_zone0/temp.")
            }
            ErrorKind::NotFound => {
                panic!("No temperature sensor found. Make sure you're running on a Raspberry Pi.")
            }
            _ => "45000".to_string(),
        },
    };
    temp_unparsed.trim().parse::<f32>().unwrap_or(45000.0) / 1000.0
}

/// The custom fan curve that determines the speed that the fan should be at
/// based on the temperature reported back by the Raspberry Pi
#[inline]
fn fan_curve(temp: f32) -> f32 {
    (0.5 * (1.0 - ((PI * temp) / 50.0).sin())
        + (FAN_LOW + ((temp - MIN_TEMP).min(MAX_TEMP) * FAN_GAIN)))
        / 2.0
}

/// Returns the fan speed as a value between 0x00 and 0xFF (MAX_SPEED)
fn handle_fan_speed(cpu_temp: f32, i2c: &mut I2c) -> Result<u8, rppal::i2c::Error> {
    let fan_percentage = match cpu_temp {
        t if t < OFF_TEMP => FAN_OFF,
        t if t < MIN_TEMP => FAN_LOW,
        t if t < MAX_TEMP => fan_curve(t),
        _ => FAN_MAX,
    };
    let fan_speed = (MAX_SPEED as f32 * fan_percentage) as u8;
    let mut old_speed = FAN_SPEED.lock().unwrap();
    if old_speed.clone() != fan_speed {
        i2c.smbus_write_byte(I2C_CMD, fan_speed)?;
        *old_speed = fan_speed;
    }

    Ok(fan_speed)
}

fn main() {
    let mut i2c = match I2c::with_bus(I2C_BUS) {
        Ok(bus) => bus,
        Err(rppal::i2c::Error::Io(e)) => match e.kind() {
            ErrorKind::PermissionDenied => {
                eprintln!("");
                std::process::exit(1);
            }
            ErrorKind::NotFound => {
                eprintln!("");
                std::process::exit(1);
            }
            _ => panic!("Error: {e}"),
        },
        Err(e) => panic!("Error: {e}"),
    };
    i2c.set_slave_address(I2C_SLA).unwrap();
    let initial_speed = i2c.smbus_read_byte(I2C_CMD).unwrap();

    {
        let mut fan_speed = FAN_SPEED.lock().unwrap();
        *fan_speed = initial_speed;
    }

    loop {
        let cpu_temp = get_cpu_temp();
        let fan_speed = handle_fan_speed(cpu_temp, &mut i2c).expect("Error setting fan speed");
        println!("CPU Temp: {cpu_temp:.2}°C, Fan Speed: {fan_speed}");

        std::thread::sleep(Duration::from_secs(5));
    }
}
