use cfg_if::cfg_if;
use fixed::types::{I8F8, I1F15, I8F24};
use fixed_macro::fixed;

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

    let mut prev_ewma: Option<I8F24> = None;

    const EXP_DECAY_30: I1F15 = fixed_macro::fixed!(0.970445: I1F15); // e^(-30.0ms / 1000.0ms)
    const EXP_DECAY_60: I1F15 = fixed_macro::fixed!(0.941764: I1F15); // e^(-60.0ms / 1000.0ms)
    const EXP_DECAY_120: I1F15 = fixed_macro::fixed!(0.886920: I1F15); // e^(-120.0ms / 1000.0ms)
    const EXP_DECAY_240: I1F15 = fixed_macro::fixed!(0.786627: I1F15); // e^(-240.0ms / 1000.0ms)

    let decay = match sample_time {
        30 => EXP_DECAY_30,
        60 => EXP_DECAY_60,
        120 => EXP_DECAY_120,
        240 => EXP_DECAY_240,
        _ => unreachable!()
    };

    (0..args.num)
        .zip(iter::repeat_with(|| tcn.temperature()))
        .map(|(i, t)| (i as f32, t.map(|t| I8F8::from(t))))
        .try_for_each(|(i, t)| {
            // https://en.wikipedia.org/wiki/Exponential_smoothing#Basic_(simple)_exponential_smoothing
            // Intermediate fixed width chosen through trial and error, looking at
            // the Linux load code as an example: https://en.wikipedia.org/wiki/Load_(computing)#Reckoning_CPU_load
            let temp = t?;
            let smooth_temp: I8F24;

            let alpha = I1F15::from_num(fixed!(1.0: I8F24) - I8F24::from_num(decay));

            match prev_ewma {
                Some(prev) => {
                    let temp_part = I8F24::from_num(alpha) * I8F24::from_num(temp);
                    let prev_part = I8F24::from_num(prev) * I8F24::from_num(decay);

                    smooth_temp = I8F24::from_num(temp_part + prev_part);
                    prev_ewma = Some(smooth_temp);
                }
                None => {
                    prev_ewma = Some(I8F24::from_num(temp));
                    smooth_temp = I8F24::from_num(temp);
                }
            }

            /* Simulate sending out 16 bit data points, even though our plot
            function accepts f32. */
            points.push((i as f32, f32::from(I8F8::from_num(smooth_temp))));
            data.push(f32::from(temp));

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
