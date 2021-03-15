use cfg_if::cfg_if;
use std::io::{Write, stdout};
use crossterm::{ExecutableCommand, QueueableCommand, cursor};

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        use linux_embedded_hal::{I2cdev, i2cdev::linux::LinuxI2CError};
        use tcn75a::*;
        use argh::FromArgs;
        use std::error::Error as ErrorTrait;
        use std::convert::TryInto;
        use std::thread::sleep;
        use std::time::Duration;

        #[derive(FromArgs)]
        #[argh(description = "plot tcn75a data")]
        struct InputArgs {
            #[argh(positional)]
            bus: String,
            #[argh(positional, from_str_fn(from_base_16))]
            addr: u8,
        }

        fn from_base_16(val: &str) -> Result<u8, String> {
            match u8::from_str_radix(val, 16) {
                Ok(v) => Ok(v),
                Err(_) => {
                    Err("Unable to convert address from base 16".into())
                }
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn main() -> Result<(), Box<dyn ErrorTrait>> {
    let args: InputArgs = argh::from_env();

    let i2c: I2cdev = I2cdev::new(args.bus)?;
    let mut tcn = Tcn75a::new(i2c, args.addr);

    let mut cfg = ConfigReg::new();

    cfg.set_resolution(Resolution::Bits9);
    cfg.set_comp_int(CompInt::Comparator);
    cfg.set_alert_polarity(AlertPolarity::ActiveHigh);
    cfg.set_fault_queue(FaultQueue::One);
    tcn.set_config_reg(cfg);

    tcn.set_reg_ptr(0).unwrap();
    let temp = tcn.temperature().unwrap();

    let temp_lo = temp + 1;
    let temp_hi = temp + 2;
    tcn.set_limits((temp_lo, temp_hi).try_into().unwrap()).unwrap();

    println!("Target temp is {} C! Press your finger against the sensor!", temp_hi);

    let mut stdout = stdout();

    loop {
        let temp = tcn.temperature().unwrap();

        stdout.execute(cursor::SavePosition);
        stdout.write(format!("Current temp is {} C.\r", temp).as_bytes());
        stdout.execute(cursor::RestorePosition);
        stdout.flush();

        sleep(Duration::from_millis(29u64)); // ~1 milli for i2c read, 30 milli for new temp.

        if temp >= temp_hi {
            break;
        }
    }

    println!("\nRelease finger from sensor! Waiting for {} C!", temp_lo);

    loop {
        let temp = tcn.temperature().unwrap();

        stdout.execute(cursor::SavePosition);
        stdout.write(format!("Current temp is {} C.\r", temp).as_bytes());
        stdout.execute(cursor::RestorePosition);
        stdout.flush();

        sleep(Duration::from_millis(29u64)); // ~1 milli for i2c read, 30 milli for new temp.

        if temp <= temp_lo {
            break;
        }
    }

    println!("\nLimits demo done!");

    // impl Drop?
    let _i2c_old = tcn.free();

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn main() {}
