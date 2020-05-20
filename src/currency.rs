use super::quota::{Quota, QuotaWrapper};
use dislog_hal::Bytes;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::{AttrProxy, KValueObject};
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalCurrency {
    /// 数字货币额度控制位
    quota_info: QuotaWrapper,
    /// 钱包公钥
    wallet_cert: CertificateSm2,
}

impl DigitalCurrency {
    pub const CURRENCY_LEN: usize = Quota::QUOTA_LEN_WITH_KVHEAD + 33;

    pub fn new(quota: QuotaWrapper, cert: CertificateSm2) -> Self {
        Self {
            quota_info: quota,
            wallet_cert: cert,
        }
    }

    pub fn get_quota_info(&self) -> &QuotaWrapper {
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

        let quota_info =
            <QuotaWrapper as KValueObject>::from_bytes(&bytes[..Quota::QUOTA_LEN_WITH_KVHEAD])
                .map_err(|_| KVObjectError::DeSerializeError)?;
        let wallet_cert =
            CertificateSm2::from_bytes(&bytes[Quota::QUOTA_LEN_WITH_KVHEAD..Self::CURRENCY_LEN])
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
    fn get_key(&self, key: &str) -> Result<Self::Byte, KVObjectError> {
        Err(KVObjectError::KeyIndexError)
    }

    // 根据key写值
    fn set_key(&mut self, _key: &str, _value: &Self::Byte) -> Result<(), KVObjectError> {
        Err(KVObjectError::KeyIndexError)
    }
}

impl KVBody for DigitalCurrency {}

pub type DigitalCurrencyWrapper = KVObject<DigitalCurrency>;

#[cfg(test)]
mod tests {

    #[test]
    fn test_issue_digitalcurrency() {
        use super::super::issue::Issue;
        use super::super::quota::QuotaWrapper;
        use super::{DigitalCurrency, DigitalCurrencyWrapper};
        use asymmetric_crypto::prelude::Keypair;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;
        use rand::thread_rng;

        // 发行机构
        let mut rng = thread_rng();
        let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate(&mut rng).unwrap();
        let cert = keypair_sm2.get_certificate();

        // 钱包
        let wallet_keypair_sm2: KeyPairSm2 = KeyPairSm2::generate(&mut rng).unwrap();
        let wallet_cert = keypair_sm2.get_certificate();

        let mut currencys = Vec::<(u64, u64)>::new();
        currencys.push((100, 1));
        currencys.push((50, 2));
        currencys.push((10, 5));

        let issue = Issue::new(currencys);
        let quotas = issue.quota_distribution(&cert);

        println!("{:?}", quotas);

        for each_quota in quotas.iter() {
            let mut quota = QuotaWrapper::new(MsgType::Quota, each_quota.clone());

            quota.fill_kvhead(&keypair_sm2).unwrap();

            let sign_bytes = quota.to_bytes();

            //DigitalCurrency
            let digital_currency = DigitalCurrencyWrapper::new(
                MsgType::Currency,
                DigitalCurrency::new(quota, wallet_cert.clone()),
            );

            println!("DigitalCurrency: {:?}", digital_currency);

            let serialized = serde_json::to_string(&digital_currency).unwrap();
            println!("serialized DigitalCurrency = {}", serialized);

            let deserialized: DigitalCurrencyWrapper = serde_json::from_str(&serialized).unwrap();
            println!("deserialized DigitalCurrency = {:?}", deserialized);

            let deserialized_obj: DigitalCurrency = deserialized.get_body().clone();
            println!("deserialized_obj = {:?}", deserialized_obj);
        }
    }
}
