use serde::{de, de::Deserialize};

pub fn default_volume() -> f32 {
    1.0
}

pub fn validate_volume<'de, D>(d: D) -> Result<f32, D::Error>
where
    D: de::Deserializer<'de>,
{
    f32::deserialize(d).and_then(|x| {
        if (0.0..=1.0).contains(&x) {
            Ok(x)
        } else {
            Err(de::Error::invalid_value(
                de::Unexpected::Float(x as f64),
                &"volume must be between 0.0 and 1.0",
            ))
        }
    })
}
