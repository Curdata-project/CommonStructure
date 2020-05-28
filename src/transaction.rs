use super::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use crate::get_rng_core;
use alloc::vec::Vec;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{ByteOrder, LittleEndian};
use chrono::prelude::Local;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易ID Hash [ 支付货币列表 | 目标钱包公钥 | 时间戳 | 随机值 ]
    txid: [u8; 32],
    /// 目标钱包公钥
    target: CertificateSm2,
    /// 支付货币列表
    currencys: Vec<DigitalCurrencyWrapper>,
}

impl Transaction {
    pub fn new(target: CertificateSm2, currencys: Vec<DigitalCurrencyWrapper>) -> Self {
        let mut rng = get_rng_core();
        let mut hasher = Sm3::default();

        for each in currencys.iter() {
            hasher.update(each.to_bytes());
        }

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
            currencys,
        }
    }

    pub fn get_txid(&self) -> &[u8; 32] {
        &self.txid
    }

    pub fn get_target(&self) -> &CertificateSm2 {
        &self.target
    }

    pub fn get_currencys(&self) -> &Vec<DigitalCurrencyWrapper> {
        &self.currencys
    }
}

impl Bytes for Transaction {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 32 + 33 + 4 {
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

        // 读取currencys
        let currencys_len = LittleEndian::read_u32(&bytes[reads_len..reads_len + 4]) as usize;
        if bytes.len() < reads_len + 4 + currencys_len * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD {
            return Err(KVObjectError::ValueValid);
        }

        let mut currencys = Vec::<DigitalCurrencyWrapper>::new();
        for i in 0..currencys_len {
            let offset = reads_len + 4 + i * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD;
            let currency = DigitalCurrencyWrapper::from_bytes(
                &bytes[offset..offset + DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD],
            )
            .map_err(|_| return KVObjectError::DeSerializeError)?;

            currencys.push(currency);
        }
        // unused
        //reads_len += 4 + currencys_len * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD;

        Ok(Self {
            txid,
            target,
            currencys,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();
        let mut buf_32 = [0; 4];

        // 写入txid
        ret.extend_from_slice(&self.txid[..]);

        // 写入target
        ret.extend_from_slice(self.target.to_bytes().as_ref());

        // 写入currencys
        LittleEndian::write_u32(&mut buf_32, self.currencys.len() as u32);
        ret.extend_from_slice(&buf_32);
        for each in self.currencys.iter() {
            ret.extend_from_slice(each.to_bytes().as_ref());
        }

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
