use sntpc::{Error, NtpContext, NtpResult, NtpTimestampGenerator, NtpUdpSocket, Result};
use std::mem::MaybeUninit;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::ops::Add;
use std::sync::{Arc, Mutex, Once};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: Duration,
}

impl NtpTimestampGenerator for StdTimestampGen {
    fn init(&mut self) {
        self.duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
    }
    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

#[derive(Debug)]
struct UdpSocketWrapper(UdpSocket);

impl NtpUdpSocket for UdpSocketWrapper {
    fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], addr: T) -> Result<usize> {
        match self.0.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }
    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
pub fn get_time_from_single_serv(serv: &str) -> Result<NtpResult> {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Unable to set UDP socket read timeout");
    let sock_wrapper = UdpSocketWrapper(socket);
    let ntp_context = NtpContext::new(StdTimestampGen::default());
    sntpc::get_time(serv, sock_wrapper, ntp_context)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldTimer {
    pub offset: i64,
}

pub struct WorldTimerWrapper {
    pub world_timer: Arc<Mutex<WorldTimer>>,
}

impl WorldTimer {
    pub fn utc_time(&self) -> chrono::DateTime<chrono::Utc> {
        let utc_now = chrono::Utc::now();
        utc_now.add(chrono::Duration::microseconds(self.offset))
    }
}
pub fn world_time() -> WorldTimer {
    *world_time_wrapper().world_timer.lock().unwrap()
}
pub fn world_time_wrapper() -> &'static WorldTimerWrapper {
    static mut WORLD_TIME_WRAPPER: MaybeUninit<WorldTimerWrapper> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    let world_time = WorldTimerWrapper {
        world_timer: Arc::new(Mutex::new(WorldTimer::default())),
    };

    // SAFETY: This is simple singleton pattern
    // it shouldn't cause any problems
    unsafe {
        ONCE.call_once(|| {
            // SAFETY: This is safe because we only write to the singleton once.
            WORLD_TIME_WRAPPER.write(world_time);
        });

        // SAFETY: This is safe because singleton is initialized inside ONCE call
        WORLD_TIME_WRAPPER.assume_init_ref()
    }
}

pub fn init_world_time() {
    let world_time = get_time(std::time::Duration::from_millis(500));
    *world_time_wrapper().world_timer.lock().unwrap() = world_time;
}

fn get_time(max_timeout: std::time::Duration) -> WorldTimer {
    let servs = [
        "pool.ntp.org:123",
        "time.google.com:123",
        "time.apple.com:123",
        "time.facebook.com:123",
        "time.fu-berlin.de:123",
        "ntp.fizyka.umk.pl:123",
    ];
    let mut avg_difference = 0;
    let mut number_of_reads = 0;

    let mut results: Vec<JoinHandle<Result<NtpResult>>> = Vec::new();
    for serv in servs.iter() {
        results.push(std::thread::spawn( || {
            get_time_from_single_serv(serv)
        }));

        println!("{}", serv);
    }

    let mut unjoined = results;

    let current_time = std::time::Instant::now();
    loop {
        let mut idxs = Vec::new();
        for i in 0..unjoined.len() {
            if unjoined[i].is_finished() {
                idxs.push(i);
            }
        }
        for i in idxs.iter().rev() {
            let el = unjoined.remove(*i);
            match el.join() {
                Ok(Ok(result)) => {
                    avg_difference += result.offset;
                    number_of_reads += 1;
                    //measurements.push(result.offset);
                    println!("Offset: {}", result.offset);
                    log::info!("Full time response: {:?}", result);
                }
                Ok(Err(_)) => {
                    log::warn!("Unable to get time from server");
                }
                Err(_) => {
                    log::warn!("Unable to join thread");
                }
            }
        }
        if unjoined.is_empty() {
            log::info!("All servers responded in time: {}ms", current_time.elapsed().as_millis());
            break;
        }

        if current_time.elapsed() > max_timeout {
            log::warn!("Don't wait for other servers");
            break;
        }
        std::thread::sleep(Duration::from_millis(1));

/*
        for result in unjoined.iter_mut() {
            if result.is_finished() {
                match result.join().unwrap() {
                    Ok(result) => {
                        avg_difference += result.offset;
                        number_of_reads += 1;
                        //measurements.push(result.offset);
                        println!("Offset: {}", result.offset);
                    }
                    Err(_) => {
                        log::warn!("Unable to get time from server");
                    }
                }
            }
        }*/
    }
    /*
    let mut avg_error = 0.0;
    if number_of_reads > 0 {
        avg_difference /= number_of_reads;

        for measurement in measurements.iter() {
            avg_error += (*measurement as f64 - avg_difference as f64).powf(2.0f64);
        }

        log::info!("Average difference: {}", avg_difference);
        log::info!(
            "Average error: {}",
            (avg_error / number_of_reads as f64).sqrt()
        );
        WorldTimer {
            offset: avg_difference,
        }
    } else {
        log::warn!("No time servers available");
        WorldTimer { offset: 0 }
    }*/

    WorldTimer { offset: 0 }
}
