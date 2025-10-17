use crate::util;
use egui_plot::PlotPoint;
use plotinator_log_if::{prelude::GeoPoint, rawplot::DataType};

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

    /// Single [`GeoPoint`] that well eventually end up on the map view
    #[inline]
    pub fn single_geopoint(topic: String, point: GeoPoint) -> Self {
        Self {
            topic,
            payload: TopicPayload::GeoPoint(point),
            ty: None,
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
    GeoPoint(GeoPoint),
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
pub struct MqttGeoPoint {
    pub topic: String,
    pub point: GeoPoint,
}
