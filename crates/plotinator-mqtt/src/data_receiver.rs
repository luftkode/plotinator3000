use crate::data::listener::MqttData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub enum MqttMessage {
    ConnectionState(ConnectionState),
    Data(MqttData),
}
