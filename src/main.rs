use rppal::i2c::I2c;
use std::f32::consts::PI;
use tokio::fs;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

/// Temperature below which to stop the fan
const OFF_TEMP: f32 = 40.0;
/// Temperature above which to start the fan
const MIN_TEMP: f32 = 45.0;
/// Temperature above which to reach full fan speed
const MAX_TEMP: f32 = 75.0;

/// The speed percentage that the fan is off at
const FAN_OFF: f32 = 0.0;
/// The speed percentage for lowest fan speed
const FAN_LOW: f32 = 0.1;
/// The speed percentage for full fan speed
const FAN_MAX: f32 = 1.0;
/// The slope of the fan speed vs temperature
const FAN_GAIN: f32 = (FAN_MAX - FAN_LOW) / (MAX_TEMP - MIN_TEMP);
/// The max speed setting
const MAX_SPEED: f32 = 255.0;

/// I2c fan control bus
const I2C_BUS: u8 = 10;
/// I2c fan control slave address
const I2C_SLA: u16 = 0x2f;
/// I2c fan control speed command
const I2C_CMD: u8 = 0x30;

/// Number of seconds between fan speed updates
const UPDATE_PERIOD: u64 = 5;

struct Measure(f32, u8);

/// The fan percentage curve
#[inline]
fn fan_curve(temp: f32) -> f32 {
    (0.5 * (1.0 - ((PI * temp) / 50.0).sin())
        + (FAN_LOW + ((temp - MIN_TEMP).min(MAX_TEMP) * FAN_GAIN)))
        / 2.0
}

/// The fan speed vs temperature
#[inline]
fn fan_speed(cpu_temp: f32) -> u8 {
    let fan_percentage = match cpu_temp {
        t if t < OFF_TEMP => FAN_OFF,
        t if t < MIN_TEMP => FAN_LOW,
        t if t < MAX_TEMP => fan_curve(t),
        _ => FAN_MAX,
    };
    (MAX_SPEED * fan_percentage).floor() as u8
}

/// The temperature of the cpu in degrees Celsius
async fn get_cpu_temp() -> Result<f32, std::io::Error> {
    let temp_unparsed = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").await?;
    Ok(temp_unparsed.trim().parse::<f32>().unwrap_or(45000.0) / 1000.0)
}

/// Update fan speed each PERIOD seconds
async fn fan_handle(tx: tokio::sync::mpsc::Sender<Measure>, cancel: CancellationToken) {
    let mut last_speed: u8 = 0;
    let bus = task::spawn_blocking(|| I2c::with_bus(I2C_BUS))
        .await
        .unwrap();
    if bus.is_err() {
        eprintln!("Unable to open I2c bus: {I2C_BUS}");
        cancel.cancel();
        return;
    }
    let mut i2c = bus.unwrap();
    let i2c = task::spawn_blocking(move || {
        if let Err(e) = i2c.set_slave_address(I2C_SLA) {
            Err(e)
        } else {
            Ok(i2c)
        }
    })
    .await
    .unwrap();
    if i2c.is_err() {
        eprintln!("Unable to set slave address {I2C_SLA} in I2c bus: {I2C_BUS}");
        cancel.cancel();
        return;
    }
    let mut i2c = i2c.unwrap();
    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(UPDATE_PERIOD)) => {
                if let Ok(temp) = get_cpu_temp().await {
                    let new_speed = fan_speed(temp);
                    if new_speed != last_speed {
                        let result = task::spawn_blocking(move || {
                            if let Err(e) = i2c.smbus_write_byte(I2C_CMD, new_speed) {
                                Err(e)
                            } else {
                                Ok(i2c)
                            }
                        })
                        .await
                        .unwrap();
                        if result.is_err() {
                            eprintln!("Unable to set fan speed on slave address {I2C_SLA} in I2c bus: {I2C_BUS}");
                            cancel.cancel();
                            return;
                        }
                        i2c = result.unwrap();
                        last_speed = new_speed;
                        tx.send(Measure(temp, new_speed)).await.unwrap();
                    }
                } else {
                    eprintln!("Missing cpu temperature measure!");
                    cancel.cancel();
                    break;
                }
            }
            _ = cancel.cancelled() => {
                println!("Fan control stopped.");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sig = signal(SignalKind::terminate())?;
    let (tx, mut rx) = mpsc::channel(1);
    let cancel = CancellationToken::new();
    let cloned_cancel = cancel.clone();
    let job = task::spawn(fan_handle(tx, cloned_cancel));
    loop {
        tokio::select! {
            _ = sig.recv() => {
                println!("Service stopped.");
                cancel.cancel();
                break;
                }
            _ = cancel.cancelled() => {
                eprintln!("Exit on error!");
                break;
            }
            Some(Measure(temp, speed)) = rx.recv() => {
                println!("Cpu Temp: {temp:.2}°C, Fan Speed: {speed}");

            }
        }
    }
    job.await.unwrap();
    Ok(())
}
