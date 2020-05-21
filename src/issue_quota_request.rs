use super::quota_control_field::QuotaControlField;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
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
use std::io::Cursor;

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

        let mut issue_info_b = Vec::<u8>::new();

        for each in issue_info.iter() {
            issue_info_b.write_u64::<LittleEndian>(each.0).unwrap();
            issue_info_b.write_u64::<LittleEndian>(each.1).unwrap();
        }

        let mut hasher = Sm3::default();
        hasher.update(&issue_info_b);
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

        let mut issue_id = [0u8; 32];
        issue_id.clone_from_slice(&bytes[0..IssueQuotaRequest::ISSUE_INFO_OFFSET]);

        let mut c = Cursor::new(&bytes[IssueQuotaRequest::ISSUE_INFO_OFFSET..]);
        let issue_len = c.read_u32::<LittleEndian>().unwrap();

        if bytes.len() != (32 + 4 + issue_len * 16 + 33) as usize {
            return Err(KVObjectError::ValueValid);
        }

        let mut issue_info = Vec::<(u64, u64)>::new();
        for _ in 0..issue_len {
            let value = c.read_u64::<LittleEndian>().unwrap();
            let amount = c.read_u64::<LittleEndian>().unwrap();
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

        ret.extend_from_slice(&self.issue_id[..]);

        ret.write_u32::<LittleEndian>(self.issue_info.len() as u32)
            .unwrap();
        for each in self.issue_info.iter() {
            ret.write_u64::<LittleEndian>(each.0).unwrap();
            ret.write_u64::<LittleEndian>(each.1).unwrap();
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_issue_wrapper() {
        use super::{IssueQuotaRequest, IssueQuotaRequestWrapper};
        use asymmetric_crypto::prelude::Keypair;
        use dislog_hal::Bytes;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;

        let keypair_cms: KeyPairSm2 = KeyPairSm2::generate_from_seed([
            3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
            33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
        ])
        .unwrap();

        let mut issue_info = Vec::<(u64, u64)>::new();
        issue_info.push((10, 5));
        issue_info.push((50, 2));
        issue_info.push((100, 1));
        let mut issue_quota = IssueQuotaRequestWrapper::new(
            MsgType::IssueQuotaRequest,
            IssueQuotaRequest::new(issue_info, keypair_cms.get_certificate()),
        );

        issue_quota.fill_kvhead(&keypair_cms).unwrap();

        let sign_bytes = issue_quota.to_bytes();

        let read_issue = IssueQuotaRequestWrapper::from_bytes(&sign_bytes).unwrap();

        assert_eq!(read_issue.verfiy_kvhead().is_ok(), true);

        let serialized = serde_json::to_string(&read_issue).unwrap();

        let deserialized: IssueQuotaRequestWrapper = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.verfiy_kvhead().is_ok(), true);
        assert_eq!(
            deserialized.get_body().get_issue_id(),
            issue_quota.get_body().get_issue_id()
        );
        assert_eq!(
            serde_json::to_string(&issue_quota.get_body().get_delivery_system()).unwrap(),
            serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
        );

        assert_eq!(3, deserialized.get_body().get_issue_info().len());
        assert_eq!(
            &(10u64, 5u64),
            deserialized.get_body().get_issue_info().get(0).unwrap()
        );
        assert_eq!(
            &(50u64, 2u64),
            deserialized.get_body().get_issue_info().get(1).unwrap()
        );
        assert_eq!(
            &(100u64, 1u64),
            deserialized.get_body().get_issue_info().get(2).unwrap()
        );
    }

    #[test]
    fn test_issue_quota() {
        use super::super::quota_control_field::QuotaControlFieldWrapper;
        use super::IssueQuotaRequest;
        use asymmetric_crypto::prelude::Keypair;
        use dislog_hal::Bytes;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;

        // 中心管理系统
        let keypair_cms: KeyPairSm2 = KeyPairSm2::generate_from_seed([
            3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
            33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
        ])
        .unwrap();

        // 货币发行系统
        let keypair_dcds: KeyPairSm2 = KeyPairSm2::generate_from_seed([
            3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
            33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
        ])
        .unwrap();

        let mut issue_info = Vec::<(u64, u64)>::new();
        issue_info.push((10, 5));
        issue_info.push((50, 2));
        issue_info.push((100, 1));

        let issue_quota_request =
            IssueQuotaRequest::new(issue_info, keypair_dcds.get_certificate());
        let quotas = issue_quota_request.quota_distribution();

        assert_eq!(8, quotas.len());

        for (index, quota) in quotas.iter().enumerate() {
            let mut quota_control_field =
                QuotaControlFieldWrapper::new(MsgType::QuotaControlField, quota.clone());

            assert_eq!(
                match index {
                    0 | 1 | 2 | 3 | 4 => 10,
                    5 | 6 => 50,
                    7 => 100,
                    _ => panic!("error value"),
                },
                quota_control_field.get_body().get_value()
            );

            quota_control_field.fill_kvhead(&keypair_cms).unwrap();
            let sign_bytes = quota_control_field.to_bytes();

            let read_quota = QuotaControlFieldWrapper::from_bytes(&sign_bytes).unwrap();
            assert_eq!(read_quota.verfiy_kvhead().is_ok(), true);

            let serialized = serde_json::to_string(&read_quota).unwrap();

            let deserialized: QuotaControlFieldWrapper = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                quota_control_field.get_body().get_id(),
                deserialized.get_body().get_id()
            );
            assert_eq!(
                quota_control_field.get_body().get_timestamp(),
                deserialized.get_body().get_timestamp()
            );
            assert_eq!(
                quota_control_field.get_body().get_value(),
                deserialized.get_body().get_value()
            );
            assert_eq!(
                serde_json::to_string(&quota_control_field.get_body().get_delivery_system())
                    .unwrap(),
                serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
            );
            assert_eq!(
                quota_control_field.get_body().get_trade_hash(),
                deserialized.get_body().get_trade_hash()
            );
        }
    }
}
