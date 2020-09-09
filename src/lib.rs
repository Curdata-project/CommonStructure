
pub mod digital_currency;
use serde::{Deserialize, Serializer, Deserializer};
use hex::{FromHex, ToHex};


pub fn ser_bytes_with<S>(obj: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&obj.encode_hex_upper::<String>())
}

pub fn deser_bytes_with<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    let d_str = String::deserialize(deserializer)
        .map_err(|_| serde::de::Error::custom(format_args!("invalid hex string")))?;
    let field = <[u8; 32]>::from_hex(d_str)
        .map_err(|_| serde::de::Error::custom(format_args!("invalid hex string")))?;
    Ok(field)
}