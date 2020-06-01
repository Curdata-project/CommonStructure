use super::quota_control_field::QuotaControlFieldWrapper;
use crate::get_rng_core;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{ByteOrder, LittleEndian};
use chrono::prelude::Local;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::{AttrProxy, KValueObject};
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaRecycleReceipt {
    /// 回收请求ID，256bit，根据Hash[ 额度发行信息 | 发行系统证书 | 时间戳 | 随机值 ]
    recycle_id: [u8; 32],
    /// Vec<面值, 数目>，二元组根据面值从小到大排列，且以面值索引唯一
    recycle_info: Vec<(u64, u64)>,
    /// 发行系统的sm2证书
    delivery_system: CertificateSm2,
}

impl QuotaRecycleReceipt {
    ///长度: 回收请求ID + 额度发行信息(4 + N * 16) + 发行系统的sm2证书
    //pub const QUOTA_RECYCLE_RECEIPT_LEN: usize = 32 + 4 + N * 16 + 33;
    pub const RECYCLE_INFO_OFFSET: usize = 32;

    fn new(recycle_info: Vec<(u64, u64)>, delivery_system: CertificateSm2) -> Self {
        let mut rng = get_rng_core();

        let mut hasher = Sm3::default();

        for each in recycle_info.iter() {
            hasher.update(each.0.to_le_bytes());
            hasher.update(each.0.to_le_bytes());
        }

        hasher.update(delivery_system.to_bytes().as_ref());

        let now = Local::now();
        let timestamp = now.timestamp_millis();
        hasher.update(timestamp.to_le_bytes());

        let mut arr = [0u8; 32];
        rng.fill_bytes(&mut arr);
        hasher.update(arr);
        let recycle_id = hasher.finalize();
        Self {
            recycle_id,
            recycle_info,
            delivery_system,
        }
    }

    pub fn get_recycle_id(&self) -> &[u8; 32] {
        &self.recycle_id
    }

    pub fn get_recycle_info(&self) -> &Vec<(u64, u64)> {
        &self.recycle_info
    }

    pub fn get_delivery_system(&self) -> &CertificateSm2 {
        &self.delivery_system
    }
    /// 发行系统根据进行额度回收
    pub fn recycle(
        quotas: &Vec<QuotaControlFieldWrapper>,
        delivery_system: CertificateSm2,
    ) -> Result<Self, KVObjectError> {
        let mut counter = BTreeMap::<u64, u64>::new();
        let mut recycle_info = Vec::<(u64, u64)>::new();

        for quota_control_field in quotas {
            if quota_control_field.verfiy_kvhead().is_err() {
                return Err(KVObjectError::KVHeadVerifyError);
            }

            let value_ = quota_control_field.get_body().get_value();
            if let Some(cnt) = counter.get_mut(&value_) {
                *cnt += 1;
            } else {
                counter.insert(value_, 1);
            }
        }
        for (value, amount) in counter {
            recycle_info.push((value, amount));
        }

        Ok(Self::new(recycle_info, delivery_system))
    }
}

impl Bytes for QuotaRecycleReceipt {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 36 {
            return Err(KVObjectError::ValueValid);
        }
        let mut read_offset: usize = 0;

        let mut recycle_id = [0u8; 32];
        recycle_id.clone_from_slice(&bytes[0..QuotaRecycleReceipt::RECYCLE_INFO_OFFSET]);
        read_offset += QuotaRecycleReceipt::RECYCLE_INFO_OFFSET;

        let issue_len = LittleEndian::read_u32(&bytes[read_offset..read_offset + 4]);
        read_offset += 4;

        if bytes.len() != (32 + 4 + issue_len * 16 + 33) as usize {
            return Err(KVObjectError::ValueValid);
        }

        let mut recycle_info = Vec::<(u64, u64)>::new();
        for _ in 0..issue_len {
            let value = LittleEndian::read_u64(&bytes[read_offset..read_offset + 8]);
            let amount = LittleEndian::read_u64(&bytes[read_offset + 8..read_offset + 16]);

            read_offset += 16;
            recycle_info.push((value, amount));
        }

        let delivery_system_offset = (32 + 4 + issue_len * 16u32) as usize;
        let delivery_system_end = (32 + 4 + issue_len * 16u32 + 33) as usize;
        let mut delivery_system_ = [0u8; 33];
        delivery_system_.clone_from_slice(&bytes[delivery_system_offset..delivery_system_end]);
        let delivery_system = CertificateSm2::from_bytes(&delivery_system_[..])
            .map_err(|_| return KVObjectError::DeSerializeError)?;

        Ok(Self {
            recycle_id,
            recycle_info,
            delivery_system,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();
        let mut buf_32 = [0; 4];
        let mut buf_64 = [0; 8];

        ret.extend_from_slice(&self.recycle_id[..]);

        LittleEndian::write_u32(&mut buf_32, self.recycle_info.len() as u32);
        ret.extend_from_slice(&buf_32);

        for each in self.recycle_info.iter() {
            LittleEndian::write_u64(&mut buf_64, each.0);
            ret.extend_from_slice(&buf_64);
            LittleEndian::write_u64(&mut buf_64, each.1);
            ret.extend_from_slice(&buf_64);
        }

        ret.extend_from_slice(self.delivery_system.to_bytes().as_ref());

        ret
    }
}

impl AttrProxy for QuotaRecycleReceipt {
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

impl KVBody for QuotaRecycleReceipt {}

pub type QuotaRecycleReceiptWrapper = KVObject<QuotaRecycleReceipt>;
