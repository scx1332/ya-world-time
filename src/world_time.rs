use std::fmt::Display;
use dns_lookup::lookup_host;
use sntpc::{Error, NtpContext, NtpResult, NtpTimestampGenerator, NtpUdpSocket, Result};
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::net::IpAddr::{V4, V6};
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
    pub fn local_time(&self) -> chrono::DateTime<chrono::Local> {
        let local_now = chrono::Local::now();
        local_now.add(chrono::Duration::microseconds(self.offset))
    }

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
    let world_time = get_time(std::time::Duration::from_millis(1000));
    *world_time_wrapper().world_timer.lock().unwrap() = world_time;
}

#[derive(Debug, Clone)]
struct ServerInfo {
    port: u16,
    ip_addr: String,
    host_name: String,
}

impl Display for ServerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} [{}]", self.ip_addr, self.port, self.host_name)
    }
}

struct Server {
    server_info: ServerInfo,
    join_handle: JoinHandle<Result<NtpResult>>,
}
struct Measurement {
    server_info: ServerInfo,
    result: NtpResult,
}

fn add_servers_from_host(time_servers: &mut Vec<ServerInfo>, host: &str) {
    match lookup_host(host) {
        Ok(ip_addrs) => {
            for ip_addr in ip_addrs {
                match ip_addr {
                    V4(addr) => {
                        log::debug!("Adding IPv4 address: {addr} resolved from {host}");
                        time_servers.push(ServerInfo {
                            port: 123,
                            ip_addr: addr.to_string(),
                            host_name: host.to_string(),
                        });
                    },
                    V6(addr) => {
                        log::debug!("Ignoring IPv6 address: {addr} resolved from {host}");
                    }
                }
            }
        }
        Err(_err) => {
            log::warn!("Unable to resolve host: {host}");
        }
    }
}

fn get_time(max_timeout: std::time::Duration) -> WorldTimer {
    const MAX_AT_ONCE: usize = 50;
    const MAX_SERVERS: usize = 100;

    let mut time_servers: Vec<ServerInfo> = vec![];
    add_servers_from_host(&mut time_servers, "time.google.com");
    add_servers_from_host(&mut time_servers, "ntp.qix.ca");
    add_servers_from_host(&mut time_servers, "ntp.nict.jp");
    add_servers_from_host(&mut time_servers, "pool.ntp.org");
    add_servers_from_host(&mut time_servers, "time.cloudflare.com");
    add_servers_from_host(&mut time_servers, "ntp.fizyka.umk.pl");
    add_servers_from_host(&mut time_servers, "time.apple.com");
    add_servers_from_host(&mut time_servers, "time.fu-berlin.de");
    add_servers_from_host(&mut time_servers, "time.facebook.com");

    let mut avg_difference = 0;
    let mut number_of_reads = 0;


    let mut measurements = Vec::new();
    if time_servers.len() > MAX_SERVERS {
        log::warn!("Too many servers, truncating to {}", MAX_SERVERS);
        time_servers.truncate(MAX_SERVERS);
    }
    let mut number_checked = 0;
    let chunked : Vec<Vec<ServerInfo>> = time_servers.chunks(MAX_AT_ONCE).map(|s| s.into()).collect();
    for chunk in chunked {
        log::info!("Checking [{}..{}] servers out of {}", number_checked, number_checked + chunk.len(), time_servers.len());
        number_checked += chunk.len();
        let mut results: Vec<Server> = Vec::new();
        for server_info in chunk {
            results.push(Server {
                server_info: server_info.clone(),
                join_handle: std::thread::spawn(move || get_time_from_single_serv(format!("{}:{}", server_info.ip_addr, server_info.port).as_str())),
            });
        }

        let mut unjoined = results;

        let current_time = std::time::Instant::now();
        loop {
            let mut idxs = Vec::new();
            for (idx, item) in unjoined.iter().enumerate() {
                if item.join_handle.is_finished() {
                    idxs.push(idx);
                }
            }
            for idx in idxs.iter().rev() {
                let el = unjoined.remove(*idx);
                match el.join_handle.join() {
                    Ok(Ok(result)) => {
                        avg_difference += result.offset;
                        number_of_reads += 1;
                        measurements.push(Measurement {
                            server_info: el.server_info,
                            result,
                        });
                    }
                    Ok(Err(_)) => {
                        log::warn!("Unable to get time from server {}", el.server_info);
                    }
                    Err(_) => {
                        log::warn!("Unable to join thread");
                    }
                }
            }
            if unjoined.is_empty() {
                log::info!(
                    "All servers responded in time: {}ms",
                    current_time.elapsed().as_millis()
                );
                break;
            }

            if current_time.elapsed() > max_timeout {
                let str_vec: Vec<ServerInfo> = unjoined.into_iter().map(|x| x.server_info).collect();
                log::warn!("Don't wait for other servers: {:?}", str_vec);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    let mut avg_error = 0.0;
    measurements.sort_by(|a, b| a.result.roundtrip.cmp(&b.result.roundtrip));
    if number_of_reads > 0 {
        avg_difference /= number_of_reads;

        for measurement in measurements.iter() {
            log::info!(
                "Server {}, Offset: {}ms, Roundtrip {}ms",
                measurement.server_info,
                measurement.result.offset as f64 / 1000.0,
                measurement.result.roundtrip as f64 / 1000.0
            );
            avg_error += (measurement.result.offset as f64 - avg_difference as f64).powf(2.0f64);
        }

        log::info!("Average difference: {}ms", avg_difference as f64 / 1000.0);
        log::info!(
            "Average error: {}ms",
            (avg_error / number_of_reads as f64).sqrt() / 1000.0
        );
        WorldTimer {
            offset: avg_difference,
        }
    } else {
        log::warn!("No time servers available");
        WorldTimer { offset: 0 }
    }
}
