extern crate i2cdev;

use futures::stream::StreamExt;
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::time::delay_for;

use i2cdev::core::*;
#[cfg(unix)]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const TEMP_SENSOR_ADDR: u16 = 0x48;

async fn http_server() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:8000").await?;

    let server = async move {
        let mut incoming = listener.incoming();

        while let Some(socket_res) = incoming.next().await {
            match socket_res {
                Ok(mut socket) => {
                    println!("Accepted connection from {:?}", socket.peer_addr());

                    tokio::spawn(async move {
                        // Split up the reading and writing parts of the
                        // socket.
                        let (mut reader, mut writer) = socket.split();

                        match tokio::io::copy(&mut reader, &mut writer).await {
                            Ok(amt) => {
                                println!("wrote {} bytes", amt);
                            }
                            Err(err) => {
                                eprintln!("IO error {:?}", err);
                            }
                        }
                    });
                }
                Err(err) => {
                    // Handle error by printing to STDOUT.
                    println!("accept error = {:?}", err);
                }
            }
        }
    };

    server.await;
    Ok(())
}

// real code should probably not use unwrap()
#[cfg(unix)]
fn i2cfun() -> Result<(), LinuxI2CError> {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-1", TEMP_SENSOR_ADDR)?;

    dev.smbus_write_byte_data(0x01, 0x60)?;

    loop {
        let raw = i16::from_be(dev.smbus_read_word_data(0x00).unwrap() as i16) >> 4;
        let cels: f32 = f32::from(raw) / 16.0;
        let fahr = 1.8 * cels + 32.0;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        println!("{:?}, {:?}, {:?}, {:?}", now, raw, cels, fahr);
        thread::sleep(Duration::from_millis(1000));
    }
}

#[cfg(windows)]
async fn i2cfun() -> Result<(), ()> {
    loop {
        println!("i2c task");
        delay_for(Duration::from_millis(1000)).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let (_, _) = tokio::join!(i2cfun(), http_server());
}
