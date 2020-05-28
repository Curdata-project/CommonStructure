use super::quota_control_field::{QuotaControlField, QuotaControlFieldWrapper};
use alloc::vec::Vec;
use dislog_hal::Bytes;
use kv_object::kv_object::{KVBody, KVObject, HEAD_TOTAL_LEN};
use kv_object::prelude::AttrProxy;
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalCurrency {
    /// 数字货币额度控制位
    quota_info: QuotaControlFieldWrapper,
    /// 钱包公钥
    wallet_cert: CertificateSm2,
}

impl DigitalCurrency {
    pub const CURRENCY_LEN: usize = QuotaControlField::QUOTA_LEN_WITH_KVHEAD + 33;
    pub const CURRENCY_LEN_WITH_KVHEAD: usize = DigitalCurrency::CURRENCY_LEN + HEAD_TOTAL_LEN;

    pub fn new(quota_control_field: QuotaControlFieldWrapper, cert: CertificateSm2) -> Self {
        Self {
            quota_info: quota_control_field,
            wallet_cert: cert,
        }
    }

    pub fn get_quota_info(&self) -> &QuotaControlFieldWrapper {
        &self.quota_info
    }

    pub fn get_wallet_cert(&self) -> &CertificateSm2 {
        &self.wallet_cert
    }
}

impl Bytes for DigitalCurrency {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != Self::CURRENCY_LEN {
            return Err(KVObjectError::DeSerializeError);
        }

        let quota_info = <QuotaControlFieldWrapper as Bytes>::from_bytes(
            &bytes[..QuotaControlField::QUOTA_LEN_WITH_KVHEAD],
        )
        .map_err(|_| KVObjectError::DeSerializeError)?;
        let wallet_cert = CertificateSm2::from_bytes(
            &bytes[QuotaControlField::QUOTA_LEN_WITH_KVHEAD..Self::CURRENCY_LEN],
        )
        .map_err(|_| KVObjectError::DeSerializeError)?;

        Ok(Self {
            quota_info,
            wallet_cert,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();

        ret.extend_from_slice(self.quota_info.to_bytes().as_ref());
        ret.extend_from_slice(self.wallet_cert.to_bytes().as_ref());

        ret
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
