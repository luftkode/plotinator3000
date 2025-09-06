use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::mqttbytes::v5::{ConnectReturnCode, Packet, Publish};
use rumqttc::v5::{Client, Connection, ConnectionError, Event, MqttOptions};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::data::listener::MqttData;
use crate::data_receiver::{ConnectionState, MqttMessage};
use crate::util::timestamped_client_id;

fn setup_client(broker_host: String, broker_port: u16) -> (Client, Connection) {
    let mut mqttoptions = MqttOptions::new(
        timestamped_client_id("plotinator3000"),
        broker_host,
        broker_port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    Client::new(mqttoptions, 10000)
}

fn subscribe(client: &Client, topics: &[String]) {
    for t in topics {
        if let Err(e) = client.subscribe(t, QoS::AtMostOnce) {
            log::error!("Subscribe error: {e}");
        }
    }
}

/// Wraps the MQTT Event loop and adds functionality to check the connection status
/// to coordinate with the client to allow for flushing all packets before disconnecting
pub struct MqttClient {
    client: Client,
    stop_flag: Arc<AtomicBool>,
    state: ConnectionState,
    topics: Vec<String>,
    connection: Connection,
    tx: Sender<MqttMessage>,
}

impl MqttClient {
    /// Create a new MQTT Event Loop
    pub fn new(
        stop_flag: Arc<AtomicBool>,
        broker_host: String,
        broker_port: u16,
        topics: Vec<String>,
        tx: Sender<MqttMessage>,
    ) -> Self {
        let (client, connection) = setup_client(broker_host, broker_port);

        Self {
            client,
            stop_flag,
            state: ConnectionState::Disconnected,
            topics,
            connection,
            tx,
        }
    }

    fn update_state(&mut self, new_state: ConnectionState) -> anyhow::Result<()> {
        log::info!("MQTT: {new_state:?}");
        if self.state == ConnectionState::Disconnected && new_state == ConnectionState::Connected {
            subscribe(&self.client, &self.topics);
        }
        self.tx.send(MqttMessage::ConnectionState(new_state))?;
        self.state = new_state;
        Ok(())
    }

    /// Spawn a thread for the event loop to run in, returning the thread handle
    pub fn spawn(self) -> thread::JoinHandle<anyhow::Result<()>> {
        thread::Builder::new()
            .name("MQTT Event loop".to_owned())
            .spawn(move || {
                self.run();
                Ok(())
            })
            .expect("Failed to spawn MQTT event loop thread")
    }

    /// Poll the event loop, necessary to receive and send messages as well as to maintain connection to the broker
    fn poll(&mut self) -> anyhow::Result<()> {
        if let Ok(notification) = self.connection.recv() {
            match notification {
                Ok(Event::Incoming(packet)) => match packet {
                    Packet::ConnAck(conn_ack) => match conn_ack.code {
                        ConnectReturnCode::Success => {
                            self.update_state(ConnectionState::Connected)?;
                        }
                        c => log::warn!("Connection attempt failed: {c:?}"),
                    },
                    Packet::Disconnect(disconnect) => {
                        log::warn!("Disconnected: {disconnect:?}");
                        self.update_state(ConnectionState::Disconnected)?;
                    }
                    Packet::Publish(p) => {
                        if let Some(mqtt_data) = handle_event_packet(&p) {
                            self.tx.send(MqttMessage::Data(mqtt_data))?;
                        }
                    }
                    _ => (), // do nothing
                },

                Err(e) => {
                    // if a connection error occurs while we are disconnecting, just break the event loop.
                    // this can happen if the broker is shutdown/connection lost and a request for stop occurs before
                    // the bad connection is detected (because of keepalive timeout)
                    log::error!("{e}");
                    self.update_state(ConnectionState::Disconnected)?;
                    // If this was an unexpected connection error, sleep a bit so we don't busy wait trying to reconnect
                    thread::sleep(Duration::from_millis(50));

                    // If there's some connection errors we want to handle specifically, we do it here
                    if let ConnectionError::ConnectionRefused(connect_return_code) = e {
                        panic!(
                            "Unexpected Connection Refused: {connect_return_code:?}. Is the broker authentication setup correct?"
                        )
                    }
                }
                Ok(Event::Outgoing(_packet)) => {}
            }
        }
        Ok(())
    }

    fn run(mut self) {
        while !self.stop_flag.load(Ordering::Relaxed) {
            if let Err(e) = self.poll() {
                log::error!("{e}, shutting down MQTT listener...");
                break;
            }
        }
    }
}

impl Drop for MqttClient {
    fn drop(&mut self) {
        let _ = self.update_state(ConnectionState::Disconnected);
    }
}

pub(crate) fn handle_event_packet(packet: &Publish) -> Option<MqttData> {
    let topic = String::from_utf8_lossy(&packet.topic);
    let payload = String::from_utf8_lossy(&packet.payload);
    log::debug!("Received on topic={topic}, payload={payload}");

    crate::parse_packet::parse_packet(&topic, &payload)
}
