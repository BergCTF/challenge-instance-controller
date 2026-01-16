use chrono::{DateTime as ChronoDateTime, SecondsFormat, Utc};
use schemars::{json_schema, JsonSchema};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct DateTime(pub ChronoDateTime<Utc>);

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // .net is a bit lossy
        serializer.serialize_str(&self.0.to_rfc3339_opts(SecondsFormat::Micros, true))
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ChronoDateTime::parse_from_rfc3339(&s)
            .map(|dt| DateTime(dt.with_timezone(&Utc)))
            .map_err(de::Error::custom)
    }
}

impl JsonSchema for DateTime {
    fn schema_name() -> Cow<'static, str> {
        "DateTime".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "type": "string",
            "format": "date-time"
        })
    }
}

impl DateTime {
    pub fn now() -> Self {
        DateTime(Utc::now())
    }
}

impl From<ChronoDateTime<Utc>> for DateTime {
    fn from(dt: ChronoDateTime<Utc>) -> Self {
        DateTime(dt)
    }
}
