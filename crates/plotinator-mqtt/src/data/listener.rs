use crate::util;
use egui_plot::PlotPoint;
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

/// Received MQTT data, before it is plotable.
///
/// May contain multiple kinds of data which each may have multiple plotable points
///
/// e.g. if a GPS publishes longitude and latitude on the topic `dt/frame/device1234/gps`
/// it may be plotted independently as `dt/frame/gps[lat]` and `dt/frame/gps[lon]`.
///
/// Buffered data may also be published to a topic which means it contains multiple data points per message
#[derive(Debug)]
pub struct MqttData {
    pub inner: MqttTopicDataWrapper,
}

impl MqttData {
    #[inline]
    pub(crate) fn single(topic_data: MqttTopicData) -> Self {
        Self {
            inner: topic_data.into(),
        }
    }

    #[inline]
    pub(crate) fn multiple(topic_data: Vec<MqttTopicData>) -> Self {
        Self {
            inner: topic_data.into(),
        }
    }
}

#[derive(Debug)]
pub enum MqttTopicDataWrapper {
    Topic(MqttTopicData),
    Topics(Vec<MqttTopicData>),
}

impl From<MqttTopicData> for MqttTopicDataWrapper {
    #[inline]
    fn from(value: MqttTopicData) -> Self {
        Self::Topic(value)
    }
}

impl From<Vec<MqttTopicData>> for MqttTopicDataWrapper {
    #[inline]
    fn from(value: Vec<MqttTopicData>) -> Self {
        Self::Topics(value)
    }
}

#[derive(Debug)]
pub struct MqttTopicData {
    pub topic: String,
    pub payload: TopicPayload,
    pub ty: Option<DataType>,
}

impl MqttTopicData {
    /// Single point timestamped with the current system time
    #[inline]
    pub fn single(topic: String, value: f64) -> Self {
        Self::single_with_ts(topic, value, util::now_timestamp())
    }

    /// Single point with supplied timestamp
    #[inline]
    pub fn single_with_ts(topic: String, value: f64, timestamp: f64) -> Self {
        Self {
            topic,
            payload: PlotPoint {
                x: timestamp,
                y: value,
            }
            .into(),
            ty: None,
        }
    }

    /// Single [`GeoPoint`] that will eventually end up on the map view
    #[inline]
    pub fn single_geopoint(topic: String, point: GeoPoint) -> Self {
        Self {
            topic,
            payload: TopicPayload::GeoData(GeoData::GeoPoint(point)),
            ty: None,
        }
    }

    /// Single altitude sample from a laser range finder that will be attempted to be associated with a path in the map view
    #[inline]
    pub fn single_laser_altitude(topic: String, val: f32, device: MqttDevice) -> Self {
        Self {
            topic,
            payload: TopicPayload::GeoData(GeoData::LaserAltitude {
                timestamp: util::now_timestamp(),
                val,
                device,
            }),
            ty: Some(DataType::AltitudeLaser),
        }
    }

    #[inline]
    pub fn multiple(topic: String, points: Vec<PlotPoint>) -> Self {
        Self {
            topic,
            payload: points.into(),
            ty: None,
        }
    }

    /// Specify the type of the MQTT data
    #[inline]
    pub fn with_ty(mut self, ty: DataType) -> Self {
        self.ty = Some(ty);
        self
    }

    #[inline]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the name as it would appear in the plot area legend e.g. `/dt/tc/frame-gps/1 Velocity [km/h]`
    #[inline]
    pub fn legend(&self) -> String {
        self.ty
            .as_ref()
            .map_or_else(|| self.topic.clone(), |t| t.legend_name_mqtt(&self.topic))
    }
}

#[derive(Debug)]
pub enum TopicPayload {
    Point(PlotPoint),
    Points(Vec<PlotPoint>),
    GeoData(GeoData),
}

impl From<PlotPoint> for TopicPayload {
    #[inline]
    fn from(value: PlotPoint) -> Self {
        Self::Point(value)
    }
}

impl From<Vec<PlotPoint>> for TopicPayload {
    #[inline]
    fn from(value: Vec<PlotPoint>) -> Self {
        Self::Points(value)
    }
}

#[derive(Debug, Clone)]
pub enum GeoData {
    GeoPoint(GeoPoint),
    LaserAltitude {
        timestamp: f64,
        val: f32,
        device: MqttDevice,
    },
}

impl GeoData {
    fn has_coordinates(&self) -> bool {
        match self {
            Self::GeoPoint(_) => true,
            Self::LaserAltitude { .. } => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MqttGeoData {
    pub topic: String,
    pub device: Option<MqttDevice>,
    pub data: GeoData,
}

impl MqttGeoData {
    pub fn point(topic: String, point: GeoPoint) -> Self {
        Self::new(topic, GeoData::GeoPoint(point))
    }

    fn new(topic: String, data: GeoData) -> Self {
        Self {
            device: MqttDevice::new(&topic),
            topic,
            data,
        }
    }

    pub fn has_coordinates(&self) -> bool {
        self.data.has_coordinates()
    }
}

/// The device, e.g. `njord` in `dt/njord/njord-altimeter/x`
///
/// Topic semantics are `<category>/<device>/<application>/<id> ...`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum MqttDevice {
    Tc,
    Njord,
}

impl MqttDevice {
    /// Parse the device name (the second segment) from an MQTT topic string
    /// with format `<category>/<device>/<application>/<id> ...`
    pub fn new(topic: &str) -> Option<Self> {
        const PREFIX: &str = "dt/";
        if !topic.starts_with(PREFIX) {
            return None;
        }

        // Find the end of the device segment
        let rest = &topic[PREFIX.len()..];
        let end = rest.find('/')?;
        let device_str = &rest[..end];

        match device_str {
            "tc" => Some(Self::Tc),
            "njord" => Some(Self::Njord),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_njord_topic() {
        let topic = "dt/njord/njord-altimeter/x";
        assert_eq!(MqttDevice::new(topic), Some(MqttDevice::Njord));
    }

    #[test]
    fn test_tc_topic() {
        let topic = "dt/tc/sensor/123";
        assert_eq!(MqttDevice::new(topic), Some(MqttDevice::Tc));
    }

    #[test]
    fn test_unknown_device() {
        let topic = "dt/unknown/app/1";
        assert_eq!(MqttDevice::new(topic), None);
    }

    #[test]
    fn test_missing_segments() {
        let topic = "dt/njord";
        assert_eq!(MqttDevice::new(topic), None);
    }

    #[test]
    fn test_empty_topic() {
        let topic = "";
        assert_eq!(MqttDevice::new(topic), None);
    }
}
