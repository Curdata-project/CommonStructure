use super::quota_control_field::QuotaControlField;
use alloc::vec::Vec;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{ByteOrder, LittleEndian};
use chrono::prelude::Local;
use core::convert::AsRef;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueQuotaRequest {
    /// 发行请求ID，256bit，根据Hash[ 额度发行信息 | 发行系统证书 | 时间戳 | 随机值 ]
    issue_id: [u8; 32],
    /// 额度发行信息，Vec<面值, 数目>，二元组根据面值从小到大排列，且以面值索引唯一
    issue_info: Vec<(u64, u64)>,
    /// 发行系统的sm2证书
    delivery_system: CertificateSm2,
}

impl IssueQuotaRequest {
    ///长度: 发行请求ID + 额度发行信息(4 + N * 16) + 发行系统的sm2证书
    //pub const ISSUR_QUOTA_FIELD_LEN: usize = 32 + 4 + N * 16 + 33;
    pub const ISSUE_INFO_OFFSET: usize = 32;

    pub fn new(issue_info: Vec<(u64, u64)>, delivery_system: CertificateSm2) -> Self {
        let mut rng = rand::thread_rng();

        let mut hasher = Sm3::default();

        for each in issue_info.iter() {
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
        let issue_id = hasher.finalize();
        Self {
            issue_id,
            issue_info,
            delivery_system,
        }
    }

    pub fn get_issue_id(&self) -> &[u8; 32] {
        &self.issue_id
    }

    pub fn get_issue_info(&self) -> &Vec<(u64, u64)> {
        &self.issue_info
    }

    pub fn get_delivery_system(&self) -> &CertificateSm2 {
        &self.delivery_system
    }

    pub fn quota_distribution(&self) -> Vec<QuotaControlField> {
        let mut ret = Vec::<QuotaControlField>::new();

        let mut rng = rand::thread_rng();

        let mut hasher = Sm3::default();
        hasher.update(&self.to_bytes()[..]);
        let trade_hash = hasher.finalize();
        for (value, amount) in self.issue_info.iter() {
            let now = Local::now();
            let timestamp = now.timestamp_millis();

            // 在循环中为每张货币生成唯一ID,
            // ID = Hasher[ 时间戳 | 面额 | 发行系统标识(证书) | 交易哈希 | 随机值 ]
            for _ in 0..*amount {
                let mut arr = [0u8; 32];
                rng.fill_bytes(&mut arr);
                let mut hasher = Sm3::default();
                hasher.update(timestamp.to_le_bytes());
                hasher.update(value.to_le_bytes());
                hasher.update(self.delivery_system.to_bytes().as_ref());
                hasher.update(trade_hash);
                hasher.update(arr);
                let id = hasher.finalize();

                ret.push(QuotaControlField::new(
                    id,
                    timestamp,
                    *value,
                    self.delivery_system.clone(),
                    trade_hash,
                ));
            }
        }

        ret
    }
}

impl Bytes for IssueQuotaRequest {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 36 {
            return Err(KVObjectError::ValueValid);
        }
        let mut read_offset: usize = 0;

        let mut issue_id = [0u8; 32];
        issue_id.clone_from_slice(&bytes[0..IssueQuotaRequest::ISSUE_INFO_OFFSET]);
        read_offset += IssueQuotaRequest::ISSUE_INFO_OFFSET;

        let issue_len = LittleEndian::read_u32(&bytes[read_offset..read_offset + 4]);
        read_offset += 4;

        if bytes.len() != (32 + 4 + issue_len * 16 + 33) as usize {
            return Err(KVObjectError::ValueValid);
        }

        let mut issue_info = Vec::<(u64, u64)>::new();
        for _ in 0..issue_len {
            let value = LittleEndian::read_u64(&bytes[read_offset..read_offset + 8]);
            let amount = LittleEndian::read_u64(&bytes[read_offset + 8..read_offset + 16]);

            read_offset += 16;
            issue_info.push((value, amount));
        }

        let delivery_system_offset = (32 + 4 + issue_len * 16u32) as usize;
        let delivery_system_end = (32 + 4 + issue_len * 16u32 + 33) as usize;
        let mut delivery_system_ = [0u8; 33];
        delivery_system_.clone_from_slice(&bytes[delivery_system_offset..delivery_system_end]);
        let delivery_system = CertificateSm2::from_bytes(&delivery_system_[..])
            .map_err(|_| return KVObjectError::DeSerializeError)?;

        Ok(Self {
            issue_id,
            issue_info,
            delivery_system,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();
        let mut buf_32 = [0; 4];
        let mut buf_64 = [0; 8];

        ret.extend_from_slice(&self.issue_id[..]);

        LittleEndian::write_u32(&mut buf_32, self.issue_info.len() as u32);
        ret.extend_from_slice(&buf_32);

        for each in self.issue_info.iter() {
            LittleEndian::write_u64(&mut buf_64, each.0);
            ret.extend_from_slice(&buf_64);
            LittleEndian::write_u64(&mut buf_64, each.1);
            ret.extend_from_slice(&buf_64);
        }

        ret.extend_from_slice(self.delivery_system.to_bytes().as_ref());

        ret
    }
}

impl AttrProxy for IssueQuotaRequest {
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

impl KVBody for IssueQuotaRequest {}

pub type IssueQuotaRequestWrapper = KVObject<IssueQuotaRequest>;
