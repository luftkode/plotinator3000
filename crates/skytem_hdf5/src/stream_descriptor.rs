use std::{collections::HashMap, io};

use serde::{Deserialize, Serialize};
use toml::Value;

use crate::util::read_string_attribute;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct StreamDescriptor {
    stream_id: String,
    chunk_size: Vec<i32>,
    description: String,
    unit: String,
    data_type: String,
    timestamp_stream: String,
    axes: HashMap<String, Axis>,
    converter: Converter,
    aux_metadata: AuxMetadata,
}

impl StreamDescriptor {
    /// Flattens the [`StreamDescriptor`] to a list of key-value pairs
    pub(crate) fn to_metadata(&self) -> Vec<(String, String)> {
        let mut metadata = Vec::new();

        if !self.stream_id.is_empty() {
            metadata.push(("stream_id".to_owned(), self.stream_id.clone()));
        }
        if !self.chunk_size.is_empty() {
            metadata.push(("chunk_size".to_owned(), format!("{:?}", self.chunk_size)));
        }
        if !self.description.is_empty() {
            metadata.push(("description".to_owned(), self.description.clone()));
        }
        if !self.unit.is_empty() {
            metadata.push(("unit".to_owned(), self.unit.clone()));
        }
        if !self.data_type.is_empty() {
            metadata.push(("data_type".to_owned(), self.data_type.clone()));
        }
        if !self.timestamp_stream.is_empty() {
            metadata.push(("timestamp_stream".to_owned(), self.timestamp_stream.clone()));
        }

        // Flatten axes
        for (key, axis) in &self.axes {
            for (sub_key, value) in axis.to_metadata() {
                if !value.is_empty() {
                    metadata.push((format!("axes.{key}.{sub_key}"), value));
                }
            }
        }

        // Converter
        for (key, value) in self.converter.to_metadata() {
            if !value.is_empty() {
                metadata.push((format!("converter.{key}"), value));
            }
        }

        // AuxMetadata
        for (key, value) in self.aux_metadata.to_metadata() {
            if !value.is_empty() {
                metadata.push((format!("aux_metadata.{key}"), value));
            }
        }

        metadata
    }
}

impl TryFrom<&hdf5::Dataset> for StreamDescriptor {
    type Error = io::Error;

    fn try_from(dataset: &hdf5::Dataset) -> Result<Self, Self::Error> {
        let sd_attr = dataset.attr("stream_descriptor")?;
        let sd_toml = read_string_attribute(&sd_attr)?;

        let Ok(sd) = toml::from_str(&sd_toml) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed decoding 'stream_descriptor' string as TOML: {sd_toml}"),
            ));
        };
        Ok(sd)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Axis {
    classname: String,
    description: String,
    values: Vec<Value>,
    unit: String,
}

impl Axis {
    fn to_metadata(&self) -> Vec<(String, String)> {
        let values_str: String = self
            .values
            .iter()
            .map(|v| match v {
                Value::String(s) => s.to_owned(),
                Value::Integer(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Datetime(datetime) => datetime.to_string(),
                Value::Array(values) => format!("{values:?}"),
                Value::Table(map) => format!("{map:?}"),
            })
            .collect::<Vec<String>>()
            .join(",");

        let mut metadata = vec![];
        if !self.classname.is_empty() {
            metadata.push(("classname".to_owned(), self.classname.clone()));
        }
        if !self.description.is_empty() {
            metadata.push(("description".to_owned(), self.description.clone()));
        }
        if !values_str.is_empty() {
            metadata.push(("values".to_owned(), values_str));
        }
        if !self.unit.is_empty() {
            metadata.push(("unit".to_owned(), self.unit.clone()));
        }
        metadata
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Converter {
    classname: String,
}

impl Converter {
    fn to_metadata(&self) -> Vec<(String, String)> {
        let mut metadata = vec![];
        if !self.classname.is_empty() {
            metadata.push(("classname".to_owned(), self.classname.clone()));
        }
        metadata
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuxMetadata {
    cal_offset: Option<f64>,
    cal_scale: Option<f64>,
}

impl AuxMetadata {
    fn to_metadata(&self) -> Vec<(String, String)> {
        let mut md = vec![];
        if let Some(c) = self.cal_offset {
            md.push(("cal_offset".to_owned(), c.to_string()));
        }
        if let Some(c) = self.cal_scale {
            md.push(("cal_scale".to_owned(), c.to_string()));
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    const TEST_TOML_STR: &str = r#"stream_id = "hm_current"
    chunk_size = [
        10,
        303,
        2,
    ]
    description = "TX Loop Current"
    unit = "A"
    data_type = "numpy.float32"
    timestamp_stream = ""

    [axes.0]
    classname = "Primary"
    description = ""
    values = []
    unit = ""

    [axes.1]
    classname = "Selector"
    description = "hm_current"
    values = [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        27,
        28,
        29,
        30,
        31,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        42,
        43,
        44,
        45,
        46,
        47,
        48,
        49,
        50,
        51,
        52,
        53,
        54,
        55,
        56,
        57,
        58,
        59,
        60,
        61,
        62,
        63,
        64,
        65,
        66,
        67,
        68,
        69,
        70,
        71,
        72,
        73,
        74,
        75,
        76,
        77,
        78,
        79,
        80,
        81,
        82,
        83,
        84,
        85,
        86,
        87,
        88,
        89,
        90,
        91,
        92,
        93,
        94,
        95,
        96,
        97,
        98,
        99,
        100,
        101,
        102,
        103,
        104,
        105,
        106,
        107,
        108,
        109,
        110,
        111,
        112,
        113,
        114,
        115,
        116,
        117,
        118,
        119,
        120,
        121,
        122,
        123,
        124,
        125,
        126,
        127,
        128,
        129,
        130,
        131,
        132,
        133,
        134,
        135,
        136,
        137,
        138,
        139,
        140,
        141,
        142,
        143,
        144,
        145,
        146,
        147,
        148,
        149,
        150,
        151,
        152,
        153,
        154,
        155,
        156,
        157,
        158,
        159,
        160,
        161,
        162,
        163,
        164,
        165,
        166,
        167,
        168,
        169,
        170,
        171,
        172,
        173,
        174,
        175,
        176,
        177,
        178,
        179,
        180,
        181,
        182,
        183,
        184,
        185,
        186,
        187,
        188,
        189,
        190,
        191,
        192,
        193,
        194,
        195,
        196,
        197,
        198,
        199,
        200,
        201,
        202,
        203,
        204,
        205,
        206,
        207,
        208,
        209,
        210,
        211,
        212,
        213,
        214,
        215,
        216,
        217,
        218,
        219,
        220,
        221,
        222,
        223,
        224,
        225,
        226,
        227,
        228,
        229,
        230,
        231,
        232,
        233,
        234,
        235,
        236,
        237,
        238,
        239,
        240,
        241,
        242,
        243,
        244,
        245,
        246,
        247,
        248,
        249,
        250,
        251,
        252,
        253,
        254,
        255,
        256,
        257,
        258,
        259,
        260,
        261,
        262,
        263,
        264,
        265,
        266,
        267,
        268,
        269,
        270,
        271,
        272,
        273,
        274,
        275,
        276,
        277,
        278,
        279,
        280,
        281,
        282,
        283,
        284,
        285,
        286,
        287,
        288,
        289,
        290,
        291,
        292,
        293,
        294,
        295,
        296,
        297,
        298,
        299,
        300,
        301,
        302,
    ]
    unit = ""

    [axes.2]
    classname = "Selector"
    description = "hm_current"
    values = [
        0,
        1,
    ]
    unit = ""

    [converter]
    classname = "Unity"

    [aux_metadata]
    cal_offset = 0
    cal_scale = 0.005
    "#;

    #[test]
    fn test_deserialize() -> TestResult {
        let stream_descriptor: StreamDescriptor = toml::from_str(TEST_TOML_STR)?;

        assert_eq!(stream_descriptor.description, "TX Loop Current");
        assert_eq!(stream_descriptor.stream_id, "hm_current");
        assert_eq!(stream_descriptor.unit, "A");
        assert_eq!(
            stream_descriptor.axes["2"].values[0].as_integer().unwrap(),
            0
        );
        assert_eq!(
            stream_descriptor.axes["2"].values[1].as_integer().unwrap(),
            1
        );

        assert_eq!(stream_descriptor.aux_metadata.cal_offset.unwrap(), 0.0);
        assert_eq!(stream_descriptor.aux_metadata.cal_scale.unwrap(), 0.005);

        Ok(())
    }

    const TEST_WASP200_TOML_STR: &str = r#"stream_id = "height"
chunk_size = [
    10,
    1,
]
description = "Range"
unit = "m"
data_type = "numpy.float32"
timestamp_stream = ""

[axes.0]
classname = "Primary"
description = ""
values = []
unit = ""

[axes.1]
classname = "Selector"
description = "Component"
values = [
    "$range$",
]
unit = "['m']"

[converter]
classname = "Unity"

[aux_metadata]
"#;

    #[test]
    fn test_deserialize_wasp200() -> TestResult {
        let stream_descriptor: StreamDescriptor = toml::from_str(TEST_WASP200_TOML_STR)?;

        assert_eq!(stream_descriptor.stream_id, "height");

        Ok(())
    }
}
