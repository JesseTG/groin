use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct RequestParams {
    source_lang: Option<String>,
    target_lang: Option<String>,
    output: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct RequestBody {
    #[serde(with = "image_serialize")]
    pub(crate) image: Vec<u8>,
    pub(crate) label: String,
    pub(crate) state: InputState,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct InputState {
    pub(crate) paused: u8,
    pub(crate) b: u8,
    pub(crate) y: u8,
    pub(crate) select: u8,
    pub(crate) start: u8,
    pub(crate) up: u8,
    pub(crate) down: u8,
    pub(crate) left: u8,
    pub(crate) right: u8,
    pub(crate) a: u8,
    pub(crate) x: u8,
    pub(crate) l: u8,
    pub(crate) r: u8,
    pub(crate) l2: u8,
    pub(crate) r2: u8,
    pub(crate) l3: u8,
    pub(crate) r3: u8,
}

#[derive(Debug)]
pub(crate) struct InvalidRequestBody;

impl warp::reject::Reject for InvalidRequestBody {}

mod image_serialize {
    use serde::{de, ser};
    use std::fmt;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::de::Error;

    pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(STANDARD.encode(data).as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string that represents a base64-encoded image")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                STANDARD.decode(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}