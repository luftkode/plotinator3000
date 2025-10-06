use crate::util::timestamped_client_id;
use rumqttc::{Client, Event, MqttOptions, Packet};
use std::{
    net::{Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs as _},
    sync::mpsc::{self, Sender},
    time::{Duration, Instant},
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum BrokerStatus {
    #[default]
    None,
    Reachable,
    Unreachable(String),
    ReachableVersion(String),
}

impl BrokerStatus {
    pub fn reachable(&self) -> bool {
        match self {
            Self::Reachable | Self::ReachableVersion(_) => true,
            Self::Unreachable(_) | Self::None => false,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorStatus {
    #[default]
    Inactive,
    Connecting,
    RetrievingVersion,
}

#[derive(Default)]
pub struct BrokerValidator {
    status: ValidatorStatus,
    previous_broker_input: String,
    broker_status: BrokerStatus,
    last_input_change: Option<Instant>,
    broker_validation_receiver: Option<mpsc::Receiver<BrokerStatus>>,
}

impl BrokerValidator {
    pub fn broker_status(&self) -> &BrokerStatus {
        &self.broker_status
    }

    pub fn status(&self) -> ValidatorStatus {
        self.status
    }

    pub fn poll_broker_status(&mut self, ip: &str, port: &str) {
        let current_broker_input = format!("{ip}{port}");

        // Detect input changes
        if current_broker_input != self.previous_broker_input {
            self.previous_broker_input = current_broker_input.clone();
            self.last_input_change = Some(Instant::now());
            self.broker_status = BrokerStatus::None;
        }

        // Debounce and validate after a timeout
        if let Some(last_change) = self.last_input_change
            && last_change.elapsed() >= Duration::from_millis(500)
            && self.status() == ValidatorStatus::Inactive
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.broker_validation_receiver = Some(rx);
            self.status = ValidatorStatus::Connecting;
            self.last_input_change = None;

            spawn_validation_thread((ip, port), tx);
        }

        // Check for validation results, if we got a result we store the result and reset the check status
        if let Some(receiver) = &mut self.broker_validation_receiver
            && let Ok(result) = receiver.try_recv()
        {
            // If the broker is reachable we continue so we can resolve its version
            match result {
                BrokerStatus::Reachable => self.status = ValidatorStatus::RetrievingVersion,
                BrokerStatus::ReachableVersion(_)
                | BrokerStatus::None
                | BrokerStatus::Unreachable(_) => {
                    self.status = ValidatorStatus::Inactive;
                    self.broker_validation_receiver = None;
                }
            }
            self.broker_status = result;
        }
    }
}

fn spawn_validation_thread((ip, port): (&str, &str), tx: Sender<BrokerStatus>) {
    // Spawn validation thread
    let (cp_host, cp_port) = (ip.to_owned(), port.to_owned());
    if let Err(e) = std::thread::Builder::new()
        .name("broker-validator".into())
        .spawn(move || {
            match validate_broker(&cp_host, &cp_port) {
                Ok(addr) => {
                    // First send that it's reachable
                    if let Err(e) = tx.send(BrokerStatus::Reachable) {
                        log::error!("{e}");
                        return;
                    }

                    // Then try to get the version
                    match get_broker_version(addr) {
                        Ok(version) => {
                            if let Err(e) = tx.send(BrokerStatus::ReachableVersion(version)) {
                                log::error!("{e}");
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to get broker version: {e}");
                            // Keep the Reachable status since we at least know it's reachable
                        }
                    }
                }
                Err(e) => {
                    if let Err(e) = tx.send(BrokerStatus::Unreachable(e)) {
                        log::error!("{e}");
                    }
                }
            }
        })
    {
        log::error!("{e}");
        debug_assert!(false, "{e}");
    }
}

fn validate_broker(host: &str, port: &str) -> Result<SocketAddr, String> {
    // Validate port first
    let port: u16 = port.parse().map_err(|e| format!("Invalid port: {e}"))?;

    // Format host properly for IPv6 if needed
    let formatted_host = if let Ok(ipv6) = host.parse::<Ipv6Addr>() {
        format!("[{ipv6}]")
    } else {
        host.to_owned()
    };

    // Create proper address string
    let addr_str = format!("{formatted_host}:{port}");

    // Resolve hostname using DNS (including mDNS if supported by system)
    let addrs = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed: {e}"))?;

    // Try all resolved addresses with timeout
    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
            Ok(_) => return Ok(addr),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.map_or_else(
        || "No addresses found".to_owned(),
        |e| format!("Connection failed: {e}"),
    ))
}

fn get_broker_version(addr: SocketAddr) -> Result<String, String> {
    let client_id = timestamped_client_id("version-check");
    let mut mqttoptions = MqttOptions::new(client_id, addr.ip().to_string(), addr.port());
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (client, mut connection) = Client::new(mqttoptions, 100);

    // Subscribe to the version topic
    if let Err(e) = client.subscribe("$SYS/broker/version", rumqttc::QoS::AtMostOnce) {
        return Err(format!("Failed to subscribe to version topic: {e}"));
    }

    // Wait for the version message with a timeout
    let start = Instant::now();
    let timeout = Duration::from_secs(2);

    while start.elapsed() < timeout {
        match connection.iter().next() {
            Some(Ok(Event::Incoming(Packet::Publish(publish)))) => {
                if publish.topic == "$SYS/broker/version"
                    && let Ok(version) = String::from_utf8(publish.payload.to_vec())
                {
                    log::info!("Got broker version: {version}");
                    return Ok(version);
                }
            }
            Some(Err(e)) => return Err(format!("Connection error: {e}")),
            None => break,
            _ => (),
        }
    }

    Err("Timeout waiting for broker version".to_owned())
}
