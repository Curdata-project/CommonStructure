use super::quota_control_field::QuotaControlFieldWrapper;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::{AttrProxy, KValueObject};
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaRecycleReceipt {
    /// 发行请求ID，256bit，根据Hash[ 额度发行信息 | 发行系统证书 | 随机值 ]
    recycle_id: [u8; 32],
    /// Vec<面值, 数目>，二元组根据面值从小到大排列，且以面值索引唯一
    recycle_info: Vec<(u64, u64)>,
    /// 发行系统的sm2证书
    delivery_system: CertificateSm2,
}

impl QuotaRecycleReceipt {
    ///长度: 发行请求ID + 额度发行信息(4 + N * 16) + 发行系统的sm2证书
    //pub const ISSUR_QUOTA_FIELD_LEN: usize = 32 + 4 + N * 16 + 33;
    pub const RECYCLE_INFO_OFFSET: usize = 32;

    pub fn new(recycle_info: Vec<(u64, u64)>, delivery_system: CertificateSm2) -> Self {
        let mut rng = rand::thread_rng();

        let mut recycle_info_b = Vec::<u8>::new();

        for each in recycle_info.iter() {
            recycle_info_b.write_u64::<LittleEndian>(each.0).unwrap();
            recycle_info_b.write_u64::<LittleEndian>(each.1).unwrap();
        }

        let mut hasher = Sm3::default();
        hasher.update(&recycle_info_b);
        hasher.update(delivery_system.to_bytes().as_ref());

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

        let mut recycle_info_b = Vec::<u8>::new();

        for each in recycle_info.iter() {
            recycle_info_b.write_u64::<LittleEndian>(each.0).unwrap();
            recycle_info_b.write_u64::<LittleEndian>(each.1).unwrap();
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

        let mut recycle_id = [0u8; 32];
        recycle_id.clone_from_slice(&bytes[0..QuotaRecycleReceipt::RECYCLE_INFO_OFFSET]);

        let mut c = Cursor::new(&bytes[QuotaRecycleReceipt::RECYCLE_INFO_OFFSET..]);
        let issue_len = c.read_u32::<LittleEndian>().unwrap();

        if bytes.len() != (32 + 4 + issue_len * 16 + 33) as usize {
            return Err(KVObjectError::ValueValid);
        }

        let mut recycle_info = Vec::<(u64, u64)>::new();
        for _ in 0..issue_len {
            let value = c.read_u64::<LittleEndian>().unwrap();
            let amount = c.read_u64::<LittleEndian>().unwrap();
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

        ret.extend_from_slice(&self.recycle_id[..]);

        ret.write_u32::<LittleEndian>(self.recycle_info.len() as u32)
            .unwrap();
        for each in self.recycle_info.iter() {
            ret.write_u64::<LittleEndian>(each.0).unwrap();
            ret.write_u64::<LittleEndian>(each.1).unwrap();
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_recycle_receipt() {
        use super::super::issue_quota_request::IssueQuotaRequest;
        use super::super::quota_control_field::QuotaControlFieldWrapper;
        use super::{QuotaRecycleReceipt, QuotaRecycleReceiptWrapper};
        use asymmetric_crypto::prelude::Keypair;
        use dislog_hal::Bytes;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;

        let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
            3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
            33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
        ])
        .unwrap();

        let mut recycle_info = Vec::<(u64, u64)>::new();
        recycle_info.push((10, 5));
        recycle_info.push((50, 2));
        recycle_info.push((100, 1));

        // 发行请求
        let issue = IssueQuotaRequest::new(recycle_info, keypair_sm2.get_certificate());
        // 额度分发
        let quotas = issue.quota_distribution();

        let mut need_recycles = Vec::<QuotaControlFieldWrapper>::new();
        for each_quota in quotas.iter() {
            let mut quota_control_field =
                QuotaControlFieldWrapper::new(MsgType::QuotaControlField, each_quota.clone());

            quota_control_field.fill_kvhead(&keypair_sm2).unwrap();

            let sign_byte = quota_control_field.to_bytes();

            let read_quota = QuotaControlFieldWrapper::from_bytes(&sign_byte).unwrap();
            need_recycles.push(read_quota);
        }
        let recycle_receipt =
            QuotaRecycleReceipt::recycle(&need_recycles, keypair_sm2.get_certificate()).unwrap();

        let mut recycle_receipt =
            QuotaRecycleReceiptWrapper::new(MsgType::QuotaRecycleReceipt, recycle_receipt);

        recycle_receipt.fill_kvhead(&keypair_sm2).unwrap();

        let sign_byte = recycle_receipt.to_bytes();

        let read_recycle_receipt = QuotaRecycleReceiptWrapper::from_bytes(&sign_byte).unwrap();

        assert_eq!(read_recycle_receipt.verfiy_kvhead().is_ok(), true);

        let serialized = serde_json::to_string(&read_recycle_receipt).unwrap();

        let deserialized: QuotaRecycleReceiptWrapper = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            recycle_receipt.get_body().get_recycle_id(),
            deserialized.get_body().get_recycle_id()
        );
        assert_eq!(
            serde_json::to_string(&recycle_receipt.get_body().get_delivery_system()).unwrap(),
            serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
        );

        assert_eq!(3, deserialized.get_body().get_recycle_info().len());
        assert_eq!(
            &(10u64, 5u64),
            deserialized.get_body().get_recycle_info().get(0).unwrap()
        );
        assert_eq!(
            &(50u64, 2u64),
            deserialized.get_body().get_recycle_info().get(1).unwrap()
        );
        assert_eq!(
            &(100u64, 1u64),
            deserialized.get_body().get_recycle_info().get(2).unwrap()
        );
    }
}
