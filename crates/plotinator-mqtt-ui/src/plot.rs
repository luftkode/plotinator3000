use std::mem;

use chrono::DateTime;
use egui::Color32;
use egui_plot::PlotPoint;
use plotinator_log_if::{prelude::RawPlotCommon, rawplot::RawPlot};
use plotinator_mqtt::data::listener::{
    MqttData, MqttGeoPoint, MqttTopicData, MqttTopicDataWrapper, TopicPayload,
};
use plotinator_ui_util::{ExpectedPlotRange, auto_color_plot_area};
use smallvec::SmallVec;

use crate::serializable::{SerializableMqttPlotData, SerializableMqttPlotPoints};

/// A collection of accumulated plot points from various MQTT topics
///
/// This is the basis for all line plots from MQTT data
#[derive(Default, Clone)]
pub struct MqttPlotData {
    // We typically won't listen to a huge amount of topics
    pub(crate) mqtt_plot_data: SmallVec<[(MqttPlotPoints, Color32); 10]>,
    pub(crate) geo_data: SmallVec<[MqttGeoPoint; 10]>,
}

impl MqttPlotData {
    #[inline]
    pub(crate) fn insert_inner_data(&mut self, data: MqttTopicData) {
        if let Some((mp, _)) = self
            .mqtt_plot_data
            .iter_mut()
            .find(|(mp, _)| mp.topic == data.topic())
        {
            match data.payload {
                TopicPayload::Points(plot_points) => {
                    for p in plot_points {
                        mp.data.push(p);
                    }
                }
                TopicPayload::Point(plot_point) => mp.data.push(plot_point),
                TopicPayload::GeoPoint(point) => self.geo_data.push(MqttGeoPoint {
                    topic: data.topic,
                    point,
                }),
            }
        } else {
            let MqttTopicData { topic, payload } = data;
            match payload {
                TopicPayload::Point(p) => self.mqtt_plot_data.push((
                    MqttPlotPoints {
                        topic,
                        data: vec![p],
                    },
                    auto_color_plot_area(ExpectedPlotRange::Hundreds),
                )),
                TopicPayload::Points(data) => self.mqtt_plot_data.push((
                    MqttPlotPoints { topic, data },
                    auto_color_plot_area(ExpectedPlotRange::Hundreds),
                )),
                TopicPayload::GeoPoint(point) => self.geo_data.push(MqttGeoPoint { topic, point }),
            };
        }
    }

    #[inline]
    pub(crate) fn insert_data(&mut self, data: MqttData) {
        match data.inner {
            MqttTopicDataWrapper::Topic(mqtt_topic_data) => self.insert_inner_data(mqtt_topic_data),
            MqttTopicDataWrapper::Topics(data) => {
                for d in data {
                    self.insert_inner_data(d);
                }
            }
        }
    }

    pub fn plots(&self) -> &[(MqttPlotPoints, Color32)] {
        &self.mqtt_plot_data
    }

    /// Take all available [MQTT Geo Points](MqttGeoPoint)
    pub fn take_geo_points(&mut self) -> Option<SmallVec<[MqttGeoPoint; 10]>> {
        if self.geo_data.is_empty() {
            None
        } else {
            Some(mem::take(&mut self.geo_data))
        }
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

impl TryFrom<MqttPlotPoints> for RawPlotCommon {
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
            ExpectedPlotRange::Hundreds
        } else {
            ExpectedPlotRange::Thousands
        };
        Ok(Self::new(name, points, expected_range))
    }
}

impl From<MqttPlotData> for SerializableMqttPlotData {
    fn from(original: MqttPlotData) -> Self {
        let MqttPlotData {
            mqtt_plot_data,
            geo_data: _, // We don't store geo points as they are duplicate data that are just sent to the map view
        } = original;
        let mut first_timestamp = None;
        for (p, _) in &mqtt_plot_data {
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

        let raw_plots: Vec<RawPlotCommon> = mqtt_plot_data
            .into_iter()
            .filter_map(|(m, _)| m.try_into().ok())
            .collect();

        for rp in &raw_plots {
            debug_assert!(rp.points().len() > 1);
        }
        let raw_plots: Vec<RawPlot> = raw_plots.into_iter().map(Into::into).collect();

        Self {
            descriptive_name,
            first_timestamp,
            mqtt_plot_data: raw_plots,
        }
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
        let SerializableMqttPlotPoints { topic, data } = serializable;
        let data = data.into_iter().map(|[x, y]| PlotPoint { x, y }).collect();
        Self { topic, data }
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
