use cfg_if::cfg_if;
use fixed::types::I8F8;

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        use linux_embedded_hal::{I2cdev, i2cdev::linux::LinuxI2CError};
        use tcn75a::*;
        use argh::FromArgs;
        use textplots::{Chart, Plot, Shape};
        use std::thread::sleep;
        use std::time::Duration;
        use indicatif::{ProgressBar, ProgressStyle};
        use serde_json;
        use std::fs::File;
        use std::io::Write;
        use std::error::Error as ErrorTrait;
        use std::convert::TryInto;
        use std::iter;

        #[derive(FromArgs)]
        #[argh(description = "plot tcn75a data")]
        struct InputArgs {
            #[argh(positional)]
            bus: String,
            #[argh(positional, from_str_fn(from_base_16))]
            addr: u8,
            #[argh(option, short='n', default = "default_num_samples()", description = "number of samples to take")]
            num: u32,
            #[argh(option, short='r', default = "default_resolution()", from_str_fn(get_resolution), description = "sample resolution")]
            res: Resolution,
            #[argh(option, short='o', description = "out json file")]
            out_file: Option<String>
        }

        #[derive(Debug)]
        #[allow(dead_code)]
        enum PlotError {
            I2c(LinuxI2CError),
            Tcn75a(tcn75a::Error<I2cdev>),
            OutputError(Box<dyn ErrorTrait>)
        }

        impl From<LinuxI2CError> for PlotError {
            fn from(i2c_err: LinuxI2CError) -> PlotError {
                PlotError::I2c(i2c_err)
            }
        }

        impl From<tcn75a::Error<I2cdev>> for PlotError {
            fn from(tcn75a_err: tcn75a::Error<I2cdev>) -> PlotError {
                PlotError::Tcn75a(tcn75a_err)
            }
        }

        fn default_num_samples() -> u32 {
            100
        }

        fn default_resolution() -> Resolution {
            Resolution::Bits11
        }

        fn from_base_16(val: &str) -> Result<u8, String> {
            let no_prefix = val.trim_start_matches("0x");

            match u8::from_str_radix(no_prefix, 16) {
                Ok(v) => Ok(v),
                Err(_) => {
                    Err("Unable to convert address from base 16".into())
                }
            }
        }

        fn get_resolution(val: &str) -> Result<Resolution, String> {
            match u8::from_str_radix(val, 10) {
                Ok(r) => {
                    r.try_into().map_err(|_| "Invalid resolution (expected 9, 10, 11, or 12)".into())
                },
                _ => {
                    Err("Invalid resolution (not a base-10 number)".into())
                }
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn main() -> Result<(), PlotError> {
    let args: InputArgs = argh::from_env();

    let i2c: I2cdev = I2cdev::new(args.bus)?;
    let mut tcn = Tcn75a::new(i2c, args.addr);
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut data: Vec<f32> = Vec::new();

    let bar = ProgressBar::new(args.num as u64);
    bar.set_style(ProgressStyle::default_bar().progress_chars("#>-"));

    let mut cfg = ConfigReg::new();
    let sample_time: u16;

    cfg.set_resolution(args.res);
    sample_time = match args.res {
        Resolution::Bits9 => 30,
        Resolution::Bits10 => 60,
        Resolution::Bits11 => 120,
        Resolution::Bits12 => 240,
    };
    tcn.set_config_reg(cfg)?;

    tcn.set_reg_ptr(0)?;
    println!(
        "Capturing data (1 sample every {} milliseconds)",
        sample_time
    );

    (0..args.num)
        .zip(iter::repeat_with(|| tcn.temperature()))
        .map(|(i, t)| (i as f32, t.map(|t| f32::from(I8F8::from(t)))))
        .try_for_each(|(i, t)| {
            let temp = t?;

            points.push((i, temp));
            data.push(temp);

            sleep(Duration::from_millis((sample_time - 1).into())); // ~1 milli for i2c read.
            bar.inc(1);

            Ok::<_, PlotError>(())
        })?;

    bar.finish();

    println!(
        "\ny = {} temperature samples (1 every {} millisconds)",
        args.num, sample_time
    );
    Chart::new(120, 60, 0.0, args.num as f32)
        .lineplot(&Shape::Steps(&points))
        .display();

    let json_str = serde_json::to_string(&data).unwrap();

    if let Some(out) = args.out_file {
        let mut file = File::create(out).map_err(|e| PlotError::OutputError(Box::new(e)))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| PlotError::OutputError(Box::new(e)))?;
    } else {
        println!("\n{}", json_str);
    }

    // impl Drop?
    let _i2c_old = tcn.free();

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn main() {}
