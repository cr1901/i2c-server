use std::convert::Infallible;

#[cfg(windows)]
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::time::delay_for;
use tokio::sync::Mutex;
use std::sync::Arc;

use base64::{encode_config_slice, URL_SAFE};
use hyper::{Body, Method, Request, Response, StatusCode, Server};
use hyper::service::{make_service_fn, service_fn};

mod samples;
use samples::SampleBuf;

#[cfg(unix)]
use i2cdev::core::*;
#[cfg(unix)]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
#[cfg(unix)]
const TEMP_SENSOR_ADDR: u16 = 0x48;


async fn temp_service(req: Request<Body>, rx: Arc<Mutex<SampleBuf<i16>>>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let max_base64_size = rx.lock().await.capacity() * 4 / 3 + 4;
            let mut payload = Vec::<u8>::with_capacity(max_base64_size);

            payload.resize(max_base64_size, 0);

            let written = {
                let temp_lock = rx.lock().await;

                let temp_data = {
                    let temp_ptr = &**temp_lock as *const [i16] as *const i16 as *const u8;
                    let temp_len = temp_lock.len();
                    unsafe { std::slice::from_raw_parts(temp_ptr, temp_len * 2) }
                };

                encode_config_slice(temp_data, URL_SAFE, &mut payload)
            };

            payload.resize(written, 0);

            Ok(Response::new(Body::from(payload)))
        },

        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// real code should probably not use unwrap()
#[cfg(unix)]
async fn i2cfun(tx: Arc<Mutex<SampleBuf<i16>>>) -> Result<(), ()> {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-1", TEMP_SENSOR_ADDR).unwrap();

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

#[cfg(windows)]
async fn i2cfun(tx: Arc<Mutex<SampleBuf<i16>>>) -> Result<(), ()> {
    let mut fake_temp : i16 = -1024;

    loop {
        let now = SystemTime::now();
        thread::sleep(Duration::from_millis(1));
        let mut lock = tx.lock().await;
        lock.post(now, fake_temp).map_err(|_| ())?;

        if lock.len() == lock.capacity() {
            break;
        }

        delay_for(Duration::from_millis(1000)).await;
        fake_temp += 1;
    }

    Ok(())
}


#[tokio::main]
async fn main() {
    let i2c_tx = Arc::new(Mutex::new(SampleBuf::<i16>::new(86400, 1)));
    let i2c_rx  = Arc::clone(&i2c_tx);

    let make_svc = make_service_fn(|_conn| {
       let foo = Arc::clone(&i2c_rx);

       async {
           Ok::<_, Infallible>(service_fn(move |body: Request<Body>| {
               temp_service(body, Arc::clone(&foo))
           }))
       }
    });

    let addr = ([0, 0, 0, 0], 8000).into();
    let server = Server::bind(&addr).serve(make_svc);

    let temp_read = i2cfun(i2c_tx);

    let (_, _) = tokio::join!(temp_read, server);
}
