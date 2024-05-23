use serde::{Deserialize, Deserializer};

pub fn false_fn() -> bool {
    false
}

pub fn deserialize_checkbox<'de, D>(deser: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deser)? {
        str if str.to_lowercase() == "on" || str.to_lowercase() == "true" => Ok(true),
        str if str.to_lowercase() == "off" || str.to_lowercase() == "false" => Ok(false),
        other => Err(serde::de::Error::custom(format!(
            "Invalid checkbox bool string {}",
            other
        ))),
    }
}

/* REFERENCES:
https://stackoverflow.com/questions/70114905/how-to-deserialize-a-string-field-to-bool
https://github.com/BurntSushi/rust-csv/issues/135

https://github.com/IsiXhosa-click/isixhosa_click/blob/8c27ca67c6966e15e0f5bc2d5d468c2645bcf58e/server/src/serialization.rs
*/
