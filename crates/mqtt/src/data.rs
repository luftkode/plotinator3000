use egui_plot::PlotPoint;

use crate::util;

/// Accumulated plot points from an MQTT topic
#[derive(Debug)]
pub struct MqttPlotPoints {
    pub topic: String,
    pub data: Vec<PlotPoint>,
}

impl From<MqttTopicData> for MqttPlotPoints {
    fn from(value: MqttTopicData) -> Self {
        let data = match value.payload {
            TopicPayload::Point(plot_point) => vec![plot_point],
            TopicPayload::Points(plot_points) => plot_points,
        };
        Self {
            topic: value.topic,
            data,
        }
    }
}

/// Received MQTT data, may contain multiple kinds of data which each may have multiple plotable points
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
    pub fn single(topic_data: MqttTopicData) -> Self {
        Self {
            inner: topic_data.into(),
        }
    }

    pub fn multiple(topic_data: Vec<MqttTopicData>) -> Self {
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
    fn from(value: MqttTopicData) -> Self {
        Self::Topic(value)
    }
}

impl From<Vec<MqttTopicData>> for MqttTopicDataWrapper {
    fn from(value: Vec<MqttTopicData>) -> Self {
        Self::Topics(value)
    }
}

#[derive(Debug)]
pub struct MqttTopicData {
    topic: String,
    pub payload: TopicPayload,
}

impl MqttTopicData {
    pub fn single(topic: String, value: f64) -> Self {
        Self {
            topic,
            payload: util::point_now(value).into(),
        }
    }

    pub fn multiple(topic: String, points: Vec<PlotPoint>) -> Self {
        Self {
            topic,
            payload: points.into(),
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}

#[derive(Debug)]
pub enum TopicPayload {
    Point(PlotPoint),
    Points(Vec<PlotPoint>),
}

impl From<PlotPoint> for TopicPayload {
    fn from(value: PlotPoint) -> Self {
        Self::Point(value)
    }
}

impl From<Vec<PlotPoint>> for TopicPayload {
    fn from(value: Vec<PlotPoint>) -> Self {
        Self::Points(value)
    }
}
