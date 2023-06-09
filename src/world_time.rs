use dns_lookup::lookup_host;
use sntpc::{Error, NtpContext, NtpResult, NtpTimestampGenerator, NtpUdpSocket, Result};
use std::env;
use std::fmt::Display;
use std::mem::MaybeUninit;
use std::net::IpAddr::{V4, V6};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
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
    pub precision: Option<i64>,
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
    let world_time = get_time();
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
                    }
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

fn get_time() -> WorldTimer {
    //const MAX_AT_ONCE: usize = 50;
    //const MAX_SERVERS: usize = 100;

    let max_at_once = env::var("YA_WORLD_TIME_MAX_AT_ONCE")
        .unwrap_or("50".to_string())
        .parse::<usize>()
        .expect("YA_WORLD_TIME_MAX_AT_ONCE cannot parse to usize");
    let max_total = env::var("YA_WORLD_TIME_MAX_TOTAL")
        .unwrap_or("100".to_string())
        .parse::<usize>()
        .expect("YA_WORLD_TIME_MAX_TOTAL cannot parse to usize");
    let max_timeout = std::time::Duration::from_millis(
        env::var("YA_WORLD_TIME_MAX_TIMEOUT")
            .unwrap_or("300".to_string())
            .parse::<u64>()
            .expect("YA_WORLD_TIME_MAX_TIMEOUT cannot parse to usize"),
    );

    let mut time_servers: Vec<ServerInfo> = vec![];

    let default_hosts = vec![
        "time.google.com",
        "ntp.qix.ca",
        "ntp.nict.jp",
        "pool.ntp.org",
        "time.cloudflare.com",
        "ntp.fizyka.umk.pl",
        "time.apple.com",
        "time.fu-berlin.de",
        "time.facebook.com",
    ];

    if let Ok(time_server_hosts) = env::var("YA_WORLD_TIME_SERVER_HOSTS") {
        for serv in time_server_hosts.split(';') {
            add_servers_from_host(&mut time_servers, serv.trim());
        }
    } else {
        for serv in default_hosts {
            add_servers_from_host(&mut time_servers, serv);
        }
    }

    let mut avg_difference = 0;
    let mut number_of_reads = 0;

    let mut measurements = Vec::new();
    if time_servers.len() > max_total {
        log::warn!("Too many servers, truncating to {}", max_total);
        time_servers.truncate(max_total);
    }
    let mut number_checked = 0;
    let chunked: Vec<Vec<ServerInfo>> =
        time_servers.chunks(max_at_once).map(|s| s.into()).collect();
    for chunk in chunked {
        log::info!(
            "Checking [{}..{}] servers out of {}",
            number_checked,
            number_checked + chunk.len(),
            time_servers.len()
        );
        number_checked += chunk.len();
        let mut results: Vec<Server> = Vec::new();
        for server_info in chunk {
            results.push(Server {
                server_info: server_info.clone(),
                join_handle: std::thread::spawn(move || {
                    get_time_from_single_serv(
                        format!("{}:{}", server_info.ip_addr, server_info.port).as_str(),
                    )
                }),
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
                let str_vec: Vec<ServerInfo> =
                    unjoined.into_iter().map(|x| x.server_info).collect();
                log::debug!("Don't wait for other servers: {:?}", str_vec);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    measurements.sort_by(|a, b| a.result.roundtrip.cmp(&b.result.roundtrip));

    if number_of_reads > 0 {
        log::info!("Total of servers responded: {}", number_of_reads);
        avg_difference /= number_of_reads;

        let mut harmonic_sum = 0.0;
        let mut harmonic_norm = 0.0;
        for measurement in measurements.iter() {
            log::debug!(
                "Server {}, Offset: {}ms, Roundtrip {}ms",
                measurement.server_info,
                measurement.result.offset as f64 / 1000.0,
                measurement.result.roundtrip as f64 / 1000.0
            );
            harmonic_sum += measurement.result.offset as f64
                / (measurement.result.roundtrip as f64).powf(2.0f64);
            harmonic_norm += 1.0 / (measurement.result.roundtrip as f64).powf(2.0f64);
        }
        let harmonic_avg = harmonic_sum / harmonic_norm;
        let harmonic_error = (1.0 / harmonic_norm).sqrt();

        let additional_systematic_error = 200.0;
        let roundtrip_to_error_multiplier = 5.0;

        log::info!(
            "Difference estimation: {:.02}ms ± {:.02}ms",
            harmonic_avg / 1000.0,
            (harmonic_error / roundtrip_to_error_multiplier + additional_systematic_error) / 1000.0
        );

        WorldTimer {
            offset: avg_difference,
            precision: Some(
                (harmonic_error / roundtrip_to_error_multiplier + additional_systematic_error)
                    as i64,
            ),
        }
    } else {
        log::warn!("No time servers available");
        WorldTimer {
            offset: 0,
            precision: None,
        }
    }
}
