use super::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use crate::get_rng_core;
use alloc::vec::Vec;
use asymmetric_crypto::hasher::sm3::Sm3;
use chrono::prelude::Local;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject, MsgType};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use kv_object::prelude::KValueObject;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易ID Hash [ 支付货币列表 | 目标钱包公钥 | 时间戳 | 随机值 ]
    txid: [u8; 32],
    /// 目标钱包证书
    target: CertificateSm2,
    /// 支付货币列表
    currency: DigitalCurrencyWrapper,
}

impl Transaction {
    pub fn new(target: CertificateSm2, currency: DigitalCurrencyWrapper) -> Self {
        let mut rng = get_rng_core();
        let mut hasher = Sm3::default();

        hasher.update(currency.to_bytes());

        hasher.update(target.to_bytes().as_ref());

        let now = Local::now();
        let timestamp = now.timestamp_millis();
        hasher.update(timestamp.to_le_bytes());

        let mut arr = [0u8; 32];
        rng.fill_bytes(&mut arr);
        hasher.update(arr);
        let txid = hasher.finalize();

        Self {
            txid,
            target,
            currency,
        }
    }

    pub fn get_txid(&self) -> &[u8; 32] {
        &self.txid
    }

    pub fn get_target(&self) -> &CertificateSm2 {
        &self.target
    }

    pub fn get_currency(&self) -> &DigitalCurrencyWrapper {
        &self.currency
    }

    /// 传入外部dcds的keypair，为货币所有权转移做签名
    /// 返回新生成的货币
    pub fn trans_currency(&self, keypair: &KeyPairSm2) -> Result<DigitalCurrencyWrapper, ()> {
        let quota_control_field = self.currency.get_body().get_quota_info();
        let mut new_currency = DigitalCurrencyWrapper::new(MsgType::DigitalCurrency, DigitalCurrency::new(quota_control_field.clone(), self.target.clone()));
        
        new_currency.fill_kvhead(keypair, &mut get_rng_core()).map_err(|_| ())?;
        Ok(new_currency)
    }
}

impl Bytes for Transaction {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 32 + 33 + DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD {
            return Err(KVObjectError::ValueValid);
        }
        let mut reads_len: usize = 0;

        // 读取txid
        let mut txid = [0u8; 32];
        txid.clone_from_slice(&bytes[reads_len..reads_len + 32]);
        reads_len += 32;

        // 读取target
        let target = CertificateSm2::from_bytes(&bytes[reads_len..reads_len + 33])
            .map_err(|_| return KVObjectError::DeSerializeError)?;
        reads_len += 33;

        // 读取currency
        let currency = DigitalCurrencyWrapper::from_bytes(
                &bytes[reads_len..reads_len + DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD],
            )
            .map_err(|_| return KVObjectError::DeSerializeError)?;

        Ok(Self {
            txid,
            target,
            currency,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();

        // 写入txid
        ret.extend_from_slice(&self.txid[..]);

        // 写入target
        ret.extend_from_slice(self.target.to_bytes().as_ref());

        // 写入currency
        ret.extend_from_slice(self.currency.to_bytes().as_ref());

        ret
    }
}

impl AttrProxy for Transaction {
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

impl KVBody for Transaction {}

pub type TransactionWrapper = KVObject<Transaction>;
