use super::quota::Quota;
use asymmetric_crypto::hasher::sm3::Sm3;
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
pub struct Issue {
    /// Vec<面值, 数目>
    currencys: Vec<(u64, u64)>,
}

impl Issue {
    pub fn new(currencys: Vec<(u64, u64)>) -> Self {
        Self { currencys }
    }

    pub fn get_issue(&self) -> &Vec<(u64, u64)> {
        &self.currencys
    }
}

impl Bytes for Issue {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() % 16 != 0 {
            return Err(KVObjectError::DeSerializeError);
        }
        let mut currencys = Vec::<(u64, u64)>::new();
        let mut pos_i = 0;
        while pos_i < bytes.len() {
            let mut value_ = [0u8; 8];
            let mut amount_ = [0u8; 8];
            value_.clone_from_slice(&bytes[pos_i..pos_i + 8]);
            amount_.clone_from_slice(&bytes[pos_i + 8..pos_i + 16]);
            currencys.push((u64::from_le_bytes(value_), u64::from_le_bytes(amount_)));

            pos_i += 16;
        }
        Ok(Self { currencys })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();

        for each in self.currencys.iter() {
            ret.extend_from_slice(&each.0.to_le_bytes()[..]);
            ret.extend_from_slice(&each.1.to_le_bytes()[..]);
        }

        ret
    }
}

impl AttrProxy for Issue {
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

impl KVBody for Issue {}

pub type IssueWrapper = KVObject<Issue>;

impl Issue {
    /// 发行系统根据自身证书对发行批准信息进行额度分发
    pub fn quota_distribution(&self, cert: &CertificateSm2) -> Vec<Quota> {
        let mut ret = Vec::<Quota>::new();

        let mut rng = rand::thread_rng();

        let mut hasher = Sm3::default();
        hasher.update(&self.to_bytes()[..]);
        let trade_hash = hasher.finalize();
        for (value, amount) in self.currencys.iter() {
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
                hasher.update(cert.to_bytes().as_ref());
                hasher.update(trade_hash);
                hasher.update(arr);
                let id = hasher.finalize();

                ret.push(Quota::new(id, timestamp, *value, cert.clone(), trade_hash));
            }
        }

        ret
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_issue_wrapper() {
        use super::{Issue, IssueWrapper};
        use asymmetric_crypto::prelude::Keypair;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;
        use rand::thread_rng;

        let mut rng = thread_rng();
        let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate(&mut rng).unwrap();

        let mut currencys = Vec::<(u64, u64)>::new();
        currencys.push((100, 1));
        currencys.push((50, 2));
        currencys.push((10, 5));
        let mut issue = IssueWrapper::new(MsgType::ISSUE, Issue { currencys });

        issue.fill_kvhead(&keypair_sm2).unwrap();

        let sign_bytes = issue.to_bytes();

        println!("sigture: {:?}", sign_bytes);

        let _issue = IssueWrapper::from_bytes(&sign_bytes).unwrap();

        println!("_issue: {:?}", _issue);

        let serialized = serde_json::to_string(&issue).unwrap();
        println!("serialized = {}", serialized);

        let deserialized: IssueWrapper = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {:?}", deserialized);
    }

    #[test]
    fn test_issue_quota() {
        use super::super::quota::{Quota, QuotaWrapper};
        use super::Issue;
        use asymmetric_crypto::prelude::Keypair;
        use kv_object::kv_object::MsgType;
        use kv_object::prelude::KValueObject;
        use kv_object::sm2::KeyPairSm2;
        use rand::thread_rng;

        let mut rng = thread_rng();
        let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate(&mut rng).unwrap();
        let cert = keypair_sm2.get_certificate();

        let mut currencys = Vec::<(u64, u64)>::new();
        currencys.push((100, 1));
        currencys.push((50, 2));
        currencys.push((10, 5));

        let issue = Issue { currencys };
        let quotas = issue.quota_distribution(&cert);

        println!("{:?}", quotas);

        for each_quota in quotas.iter() {
            let mut quota = QuotaWrapper::new(MsgType::Quota, each_quota.clone());

            quota.fill_kvhead(&keypair_sm2).unwrap();

            println!("quota: {:?}", quota);

            let sign_bytes = quota.to_bytes();

            println!("sigture: {:?}", sign_bytes);

            let serialized = serde_json::to_string(&quota).unwrap();
            println!("serialized = {}", serialized);

            let deserialized: QuotaWrapper = serde_json::from_str(&serialized).unwrap();
            println!("deserialized = {:?}", deserialized);

            let deserialized_obj: Quota = deserialized.get_body().clone();
            println!("deserialized_obj = {:?}", deserialized_obj);
        }
    }
}
