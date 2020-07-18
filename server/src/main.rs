extern crate i2cdev;

use futures::stream::StreamExt;
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::time::delay_for;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::str;

use base64::{encode_config_slice, URL_SAFE};

#[cfg(unix)]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
#[cfg(unix)]
const TEMP_SENSOR_ADDR: u16 = 0x48;

async fn http_server(rx: Arc<Mutex<Vec<i16>>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:8000").await?;
    let max_base64_size = rx.lock().await.capacity() * 4 / 3 + 4;

    let server = async move {
        let mut incoming = listener.incoming();

        while let Some(socket_res) = incoming.next().await {
            match socket_res {
                Ok(mut socket) => {
                    println!("Accepted connection from {:?}", socket.peer_addr());

                    let new_rx = rx.clone();

                    tokio::spawn(async move {
                        let mut payload = Vec::<u8>::with_capacity(max_base64_size);
                        payload.resize(max_base64_size, 0);

                        let written = {
                            let temp_lock = new_rx.lock().await;

                            let temp_data = {
                                let temp_ptr = &**temp_lock as *const [i16] as *const i16 as *const u8;
                                let temp_len = temp_lock.len();
                                unsafe { std::slice::from_raw_parts(temp_ptr, temp_len * 2) }
                            };

                            encode_config_slice(temp_data, URL_SAFE, &mut payload)
                        };

                        payload.resize(written, 0);
                        println!("{}", str::from_utf8(&payload).unwrap());
                        socket.write_all(&payload).await.unwrap();
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
async fn i2cfun(tx: Arc<Mutex<Vec<i16>>>) -> Result<(), ()> {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-1", TEMP_SENSOR_ADDR).unwrap();

    dev.smbus_write_byte_data(0x01, 0x60).unwrap();

    loop {
        let before = SystemTime::now();

        // Measured: takes approx 1 millisecond.
        let raw = i16::from_be(dev.smbus_read_word_data(0x00).unwrap() as i16) >> 4;
        let cels: f32 = f32::from(raw) / 16.0;
        let fahr = 1.8 * cels + 32.0;

        let now = SystemTime::now();
        let nix_ts = now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let elapsed = now.duration_since(before).unwrap().as_micros();

        let mut lock = tx.lock().await;
        lock[0] = raw;

        //println!("{:?}, {:?}, {:?}, {:?}", now, raw, cels, fahr);
        println!("i2c task: {:?}", elapsed);
        delay_for(Duration::from_millis(1000)).await;
    }
}

#[cfg(windows)]
async fn i2cfun(tx: Arc<Mutex<Vec<i16>>>) -> Result<(), ()> {
    let mut fake_temp : i16 = -1024;

    loop {
        thread::sleep(Duration::from_millis(1));
        let mut lock = tx.lock().await;
        lock.push(fake_temp);

        //println!("i2c task: {:?}", elapsed);
        delay_for(Duration::from_millis(1000)).await;
        fake_temp += 1;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let i2c_tx = Arc::new(Mutex::new(Vec::<i16>::with_capacity(1024)));
    let i2c_rx  = Arc::clone(&i2c_tx);

    let (_, _) = tokio::join!(i2cfun(i2c_tx), http_server(i2c_rx));
}
