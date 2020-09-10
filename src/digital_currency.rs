use crate::{deser_bytes_with, get_rng_core, ser_bytes_with};
use asymmetric_crypto::hasher::sm3::Sm3;
use chrono::prelude::Local;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use hex::ToHex;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalCurrency {
    /// 唯一标识
    #[serde(
        serialize_with = "ser_bytes_with",
        deserialize_with = "deser_bytes_with"
    )]
    id: [u8; 32],
    /// 所有者
    owner: CertificateSm2,
    /// 金额
    amount: u64,
    /// 发行系统
    issue: CertificateSm2,
    /// 脚本
    script: Vec<u8>,
    /// 附加信息
    addition: Vec<u8>,
}

impl DigitalCurrency {
    pub fn new(
        owner: CertificateSm2,
        amount: u64,
        issue: CertificateSm2,
        script: Vec<u8>,
        addition: Vec<u8>,
    ) -> Self {
        let mut rng = get_rng_core();

        let mut hasher = Sm3::default();

        let now = Local::now().timestamp_millis();

        let mut arr = [0u8; 32];
        rng.fill_bytes(&mut arr);
        hasher.update(now.to_le_bytes());
        hasher.update(owner.to_bytes().as_ref());
        hasher.update(amount.to_le_bytes());
        hasher.update(issue.to_bytes().as_ref());
        hasher.update(arr);
        let id = hasher.finalize();

        Self {
            id,
            owner,
            amount,
            issue,
            script,
            addition,
        }
    }

    pub fn get_id(&self) -> &[u8; 32] {
        &self.id
    }

    pub fn get_id_str(&self) -> String {
        self.id.encode_hex_upper::<String>()
    }

    pub fn get_owner(&self) -> &CertificateSm2 {
        &self.owner
    }

    pub fn get_amount(&self) -> u64 {
        self.amount
    }

    pub fn get_issue(&self) -> &CertificateSm2 {
        &self.issue
    }

    pub fn get_script(&self) -> &Vec<u8> {
        &self.script
    }

    pub fn get_addition(&self) -> &Vec<u8> {
        &self.addition
    }
}

impl Bytes for DigitalCurrency {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes).map_err(|_| KVObjectError::DeSerializeError)
    }

    fn to_bytes(&self) -> Self::BytesType {
        bincode::serialize(self).expect("DigitalCurrency to_bytes exception")
    }
}

impl AttrProxy for DigitalCurrency {
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

impl KVBody for DigitalCurrency {}

pub type DigitalCurrencyWrapper = KVObject<DigitalCurrency>;
