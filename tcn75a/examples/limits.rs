use cfg_if::cfg_if;
use fixed::types::I8F8;
use fixed_macro::fixed;
use std::io::{stdout, Write};

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        use crossterm::{cursor, ExecutableCommand};
        use linux_embedded_hal::I2cdev;
        use tcn75a::*;
        use argh::FromArgs;
        // no_std crates don't have access to the Error trait. However, because tcn75a crate
        // error types impl Display, we can use the eyre crate to ad-hoc convert our error types
        // to ones that impl Error via the eyre! macro.
        use eyre::{eyre, Result};
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
fn main() -> Result<()> {
    let args: InputArgs = argh::from_env();

    let i2c: I2cdev = I2cdev::new(args.bus)?;
    let mut tcn = Tcn75a::new(i2c, args.addr);

    let mut cfg = ConfigReg::new();

    cfg.set_resolution(Resolution::Bits9);
    cfg.set_comp_int(CompInt::Comparator);
    cfg.set_alert_polarity(AlertPolarity::ActiveHigh);
    cfg.set_fault_queue(FaultQueue::One);
    tcn.set_config_reg(cfg)
        .map_err(|_e| eyre!("failed to set config reg"))?;

    let temp = tcn
        .temperature()
        .map_err(|_e| eyre!("failed to read a temperature"))?;

    let temp_lo: I8F8 = I8F8::from(temp) + fixed!(1: I8F8);
    let temp_hi: I8F8 = I8F8::from(temp) + fixed!(2: I8F8);
    tcn.set_limits((temp_lo, temp_hi).try_into().unwrap())
        .map_err(|_e| eyre!("failed to set temperature sensor limits"))?;

    println!(
        "Target temp is {} C! Press your finger against the sensor!",
        temp_hi
    );

    let mut stdout = stdout();

    loop {
        let temp = tcn
            .temperature()
            .map_err(|_e| eyre!("failed to read a temperature"))?;

        stdout.execute(cursor::SavePosition)?;
        stdout.write(format!("Current temp is {} C.\r", I8F8::from(temp)).as_bytes())?;
        stdout.execute(cursor::RestorePosition)?;
        stdout.flush()?;

        sleep(Duration::from_millis(29u64)); // ~1 milli for i2c read, 30 milli for new temp.

        if I8F8::from(temp) >= temp_hi {
            break;
        }
    }

    println!("\nRelease finger from sensor! Waiting for {} C!", temp_lo);

    loop {
        let temp = tcn
            .temperature()
            .map_err(|_e| eyre!("failed to read a temperature"))?;

        stdout.execute(cursor::SavePosition)?;
        stdout.write(format!("Current temp is {} C.\r", I8F8::from(temp)).as_bytes())?;
        stdout.execute(cursor::RestorePosition)?;
        stdout.flush()?;

        sleep(Duration::from_millis(29u64)); // ~1 milli for i2c read, 30 milli for new temp.

        if I8F8::from(temp) <= temp_lo {
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
