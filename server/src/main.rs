use std::convert::Infallible;

#[cfg(unix)]
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::delay_for;

use clap::{App, AppSettings, Arg, ArgGroup, ArgMatches, SubCommand};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json;

mod samples;
use samples::SampleBuf;

#[cfg(unix)]
use i2cdev::core::*;
#[cfg(unix)]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

async fn temp_service(
    req: Request<Body>,
    rx: Arc<Mutex<SampleBuf<i16>>>,
) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let sample_buf = rx.lock().await;
            Ok(Response::new(Body::from(
                serde_json::to_string(&*sample_buf).unwrap(),
            )))
        }

        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// real code should probably not use unwrap()
#[cfg(unix)]
async fn measure<P>(path: P, addr: u16, tx: Arc<Mutex<SampleBuf<i16>>>) -> Result<(), ()>
where
    P: AsRef<Path>,
{
    let mut dev = LinuxI2CDevice::new(path, addr).unwrap();

    dev.smbus_write_byte_data(0x01, 0x60).unwrap();

    loop {
        let now = SystemTime::now();

        // Measured: takes approx 1 millisecond.
        let raw = i16::from_be(dev.smbus_read_word_data(0x00).unwrap() as i16) >> 4;

        let mut lock = tx.lock().await;
        lock.post(now, raw).map_err(|_| ())?;

        delay_for(Duration::from_millis(1000)).await;
    }
}

async fn replay_synthesize(tx: Arc<Mutex<SampleBuf<i16>>>) -> Result<(), ()> {
    let mut fake_temp: i16 = -1024;

    loop {
        let now = SystemTime::now();
        thread::sleep(Duration::from_millis(1));
        let mut lock = tx.lock().await;
        lock.post(now, fake_temp)?;

        if lock.len() == lock.capacity() {
            break;
        }

        delay_for(Duration::from_millis(1000)).await;
        fake_temp += 1;
    }

    Ok(())
}

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("I2C Sensor Server")
        .version("0.1")
        .author("William D. Jones <thor0505@comcast.net>")
        .about("Low speed I2C HTTP daemon")
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("sample_rate")
                .help("Sample rate (Hz)")
                .short("s")
                .value_name("RATE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("IP_ADDRESS")
                .help("IP Address and Port")
                .default_value("0.0.0.0:8000")
                .index(1),
        )
        .subcommand(
            SubCommand::with_name("measure")
                .about("Run the server and obtain data from I2C sensors (Unix only).")
                .arg(
                    Arg::with_name("replay")
                        .help("Write data to file for replay on exit (not implemented).")
                        .short("r")
                        .value_name("FILE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("device")
                        .help("Device type to talk to (not implemented).")
                        .short("d")
                        .value_name("DEVICE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("NODE")
                        .help("I2C device node")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("I2C_ADDRESS")
                        .help("I2C device address")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("replay")
                .about("Run the server with synthesized data from a file.")
                .arg(
                    Arg::with_name("synthesis")
                        .help("Synthesize fake data without a file.")
                        .short("s"),
                )
                .arg(
                    Arg::with_name("file")
                        .help("Replay data file to read (not implemented).")
                        .index(1),
                )
                .group(
                    ArgGroup::with_name("source")
                        .args(&["file", "synthesis"])
                        .required(true),
                ),
        )
        .get_matches()
}

#[tokio::main]
async fn main() {
    let matches = parse_args();

    let i2c_tx = Arc::new(Mutex::new(SampleBuf::<i16>::new(86400, 1)));
    let i2c_rx = Arc::clone(&i2c_tx);

    let make_svc = make_service_fn(|_conn| {
        let foo = Arc::clone(&i2c_rx);

        async {
            Ok::<_, Infallible>(service_fn(move |body: Request<Body>| {
                temp_service(body, Arc::clone(&foo))
            }))
        }
    });

    let addr = matches.value_of("IP_ADDRESS").unwrap().parse().unwrap();
    let server = Server::bind(&addr).serve(make_svc);

    if let Some(matches) = matches.subcommand_matches("measure") {
        #[cfg(unix)]
        {
            let i2c_node = matches.value_of("NODE").unwrap();
            let i2c_addr =
                u16::from_str_radix(matches.value_of("I2C_ADDRESS").unwrap(), 16).unwrap();
            let (_, _) = tokio::join!(measure(i2c_node, i2c_addr, i2c_tx), server);
        }

        #[cfg(windows)]
        println!("Measure subcommand only available on Unix systems.");
    } else if let Some(matches) = matches.subcommand_matches("replay") {
        if matches.is_present("synthesis") {
            let replay_fn = replay_synthesize(i2c_tx);
            let (_, _) = tokio::join!(replay_fn, server);
        } else {
            println!("Replay from file not yet implemented.");
        }
    }
}
