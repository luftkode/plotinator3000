use chrono::DateTime;
use egui_plot::PlotPoint;
use plotinator_log_if::prelude::{ExpectedPlotRange, RawPlot};

use crate::{SerializableMqttPlotData, SerializableMqttPlotPoints};

use super::listener::{MqttData, MqttTopicData, MqttTopicDataWrapper, TopicPayload};

/// A collection of accumulated plot points from various MQTT topics
///
/// This is the basis for all line plots from MQTT data
#[derive(Default, Clone)]
pub struct MqttPlotData {
    pub(crate) mqtt_plot_data: Vec<MqttPlotPoints>,
}

impl MqttPlotData {
    pub(crate) fn insert_inner_data(&mut self, data: MqttTopicData) {
        if let Some(mp) = self
            .mqtt_plot_data
            .iter_mut()
            .find(|mp| mp.topic == data.topic())
        {
            match data.payload {
                TopicPayload::Point(plot_point) => mp.data.push(plot_point),
                TopicPayload::Points(mut plot_points) => mp.data.append(&mut plot_points),
            }
        } else {
            self.mqtt_plot_data.push(data.into());
        }
    }

    pub(crate) fn insert_data(&mut self, data: MqttData) {
        match data.inner {
            MqttTopicDataWrapper::Topic(mqtt_topic_data) => self.insert_inner_data(mqtt_topic_data),
            MqttTopicDataWrapper::Topics(mqtt_topic_data_vec) => {
                for mtd in mqtt_topic_data_vec {
                    self.insert_inner_data(mtd);
                }
            }
        }
    }

    pub fn plots(&self) -> &[MqttPlotPoints] {
        &self.mqtt_plot_data
    }
}

/// Accumulated plot points from an MQTT topic
///
/// This is the final state of received MQTT data
/// where it is plotable
#[derive(Debug, Clone, PartialEq)]
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

impl From<MqttPlotData> for SerializableMqttPlotData {
    fn from(original: MqttPlotData) -> Self {
        let mut first_timestamp = None;
        for p in &original.mqtt_plot_data {
            if p.data.len() < 2 {
                continue; // we can't plot data with less than two points
            }
            let tmp_first_ts = p.data[0].x;
            if let Some(first_ts) = first_timestamp {
                if tmp_first_ts < first_ts {
                    first_timestamp = Some(tmp_first_ts);
                }
            } else {
                first_timestamp = Some(tmp_first_ts);
            }
        }
        let first_timestamp = first_timestamp.expect("unsound condition, no timestamps");
        let first_timestamp = DateTime::from_timestamp_nanos(first_timestamp as i64);

        let ts = first_timestamp.format("%d/%m/%Y %H:%M");
        let descriptive_name = format!("MQTT {ts}");

        let raw_plots: Vec<RawPlot> = original
            .mqtt_plot_data
            .into_iter()
            .filter_map(|m| m.try_into().ok())
            .collect();

        for rp in &raw_plots {
            debug_assert!(rp.points().len() > 1);
        }

        Self {
            descriptive_name,
            first_timestamp,
            mqtt_plot_data: raw_plots,
        }
    }
}

impl TryFrom<MqttPlotPoints> for RawPlot {
    type Error = anyhow::Error;

    fn try_from(mqtt_pp: MqttPlotPoints) -> Result<Self, Self::Error> {
        if mqtt_pp.data.len() < 2 {
            anyhow::bail!("Cannot plot less than 2 points");
        }
        let name = mqtt_pp.topic;
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut points = Vec::with_capacity(mqtt_pp.data.len());
        for pp in mqtt_pp.data {
            min = pp.y.min(min);
            max = pp.y.max(max);
            points.push([pp.x, pp.y]);
        }
        let min = min.abs();
        let max = max.abs();
        let is_boolean = (max == 1. || max == 0.) && (min == 0. || min == 1.);
        let expected_range = if is_boolean {
            ExpectedPlotRange::Percentage
        } else if max < 300. && min > -300. {
            ExpectedPlotRange::OneToOneHundred
        } else {
            ExpectedPlotRange::Thousands
        };
        Ok(Self::new(name, points, expected_range))
    }
}

impl From<MqttPlotPoints> for SerializableMqttPlotPoints {
    fn from(original: MqttPlotPoints) -> Self {
        let serializable_data = original.data.into_iter().map(|p| [p.x, p.y]).collect();
        Self {
            topic: original.topic,
            data: serializable_data,
        }
    }
}
impl From<SerializableMqttPlotPoints> for MqttPlotPoints {
    fn from(serializable: SerializableMqttPlotPoints) -> Self {
        let original_data = serializable
            .data
            .into_iter()
            .map(|[x, y]| PlotPoint { x, y })
            .collect();
        Self {
            topic: serializable.topic,
            data: original_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use egui_plot::PlotPoint;

    use super::*;

    #[test]
    fn test_serialize_deserialize_helper() {
        let original_data = vec![PlotPoint { x: 1.0, y: 2.0 }, PlotPoint { x: 3.0, y: 4.0 }];

        let original_mqtt_plot_points = MqttPlotPoints {
            topic: "sensor/temperature".to_owned(),
            data: original_data,
        };

        // Convert to helper struct for serialization
        let serializable_points: SerializableMqttPlotPoints =
            original_mqtt_plot_points.clone().into();
        let serialized = serde_json::to_string(&serializable_points).unwrap();
        assert_eq!(
            serialized,
            r#"{"topic":"sensor/temperature","data":[[1.0,2.0],[3.0,4.0]]}"#
        );

        // Deserialize into helper struct
        let deserialized_serializable_points: SerializableMqttPlotPoints =
            serde_json::from_str(&serialized).unwrap();
        // Convert back to original struct
        let deserialized_mqtt_plot_points: MqttPlotPoints = deserialized_serializable_points.into();

        let expected_data = vec![PlotPoint { x: 1.0, y: 2.0 }, PlotPoint { x: 3.0, y: 4.0 }];
        assert_eq!(deserialized_mqtt_plot_points.topic, "sensor/temperature");
        assert_eq!(deserialized_mqtt_plot_points.data, expected_data);

        assert_eq!(original_mqtt_plot_points, deserialized_mqtt_plot_points);
    }
}
