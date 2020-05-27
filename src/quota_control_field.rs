use alloc::string::String;
use alloc::vec::Vec;
use dislog_hal::Bytes;
use hex::{FromHex, ToHex};
use kv_object::kv_object::{KVBody, KVObject, HEAD_TOTAL_LEN};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaControlField {
    /// 唯一标识
    #[serde(
        serialize_with = "ser_bytes_with",
        deserialize_with = "deser_bytes_with"
    )]
    id: [u8; 32],
    /// 时间戳
    timestamp: i64,
    /// 面额
    value: u64,
    /// 发行系统的sm2证书
    delivery_system: CertificateSm2,
    /// 交易哈希
    #[serde(
        serialize_with = "ser_bytes_with",
        deserialize_with = "deser_bytes_with"
    )]
    trade_hash: [u8; 32],
}

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

impl QuotaControlField {
    ///长度: 唯一标识 + 时间戳 + 面额 + 发行系统证书 + 交易哈希
    pub const QUOTA_LEN: usize = 32 + 8 + 8 + 33 + 32;
    pub const QUOTA_LEN_WITH_KVHEAD: usize = HEAD_TOTAL_LEN + 32 + 8 + 8 + 33 + 32;

    pub fn new(
        id: [u8; 32],
        timestamp: i64,
        value: u64,
        delivery_system: CertificateSm2,
        trade_hash: [u8; 32],
    ) -> Self {
        Self {
            id,
            timestamp,
            value,
            delivery_system,
            trade_hash,
        }
    }

    pub fn get_id(&self) -> &[u8; 32] {
        &self.id
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn get_value(&self) -> u64 {
        self.value
    }

    pub fn get_delivery_system(&self) -> &CertificateSm2 {
        &self.delivery_system
    }

    pub fn get_trade_hash(&self) -> &[u8; 32] {
        &self.trade_hash
    }
}

impl Bytes for QuotaControlField {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != Self::QUOTA_LEN {
            return Err(KVObjectError::DeSerializeError);
        }
        let mut id_ = [0u8; 32];
        let mut timestamp_ = [0u8; 8];
        let mut value_ = [0u8; 8];
        let mut trade_hash_ = [0u8; 32];

        id_.clone_from_slice(&bytes[..32]);
        timestamp_.clone_from_slice(&bytes[32..40]);
        value_.clone_from_slice(&bytes[40..48]);
        trade_hash_.clone_from_slice(&bytes[81..Self::QUOTA_LEN]);

        let delivery_system = CertificateSm2::from_bytes(&bytes[48..81])
            .map_err(|_| KVObjectError::DeSerializeError)?;

        Ok(Self {
            id: id_,
            timestamp: i64::from_le_bytes(timestamp_),
            value: u64::from_le_bytes(value_),
            delivery_system,
            trade_hash: trade_hash_,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();

        ret.extend_from_slice(&self.id[..]);
        ret.extend_from_slice(&self.timestamp.to_le_bytes()[..]);
        ret.extend_from_slice(&self.value.to_le_bytes()[..]);
        ret.extend_from_slice(self.delivery_system.to_bytes().as_ref());
        ret.extend_from_slice(&self.trade_hash[..]);

        ret
    }
}

impl AttrProxy for QuotaControlField {
    type Byte = Vec<u8>;

    // 根据key读取值
    fn get_key(&self, _: &str) -> Result<Self::Byte, KVObjectError> {
        Err(KVObjectError::KeyIndexError)
    }

    // 根据key写值
    fn set_key(&mut self, _key: &str, _value: &Self::Byte) -> Result<(), KVObjectError> {
        Err(KVObjectError::KeyIndexError)
    }
}

impl KVBody for QuotaControlField {}

pub type QuotaControlFieldWrapper = KVObject<QuotaControlField>;
