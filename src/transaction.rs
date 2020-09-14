use crate::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use crate::error::CommStructError;
use asymmetric_crypto::hasher::sm3::Sm3;
use asymmetric_crypto::prelude::Certificate;
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::AttrProxy;
use kv_object::prelude::KValueObject;
use kv_object::sm2::{CertificateSm2, KeyPairSm2, SignatureSm2};
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 输入交易货币
    inputs: Vec<DigitalCurrencyWrapper>,
    /// 金额 （收款方证书, 收款金额）
    outputs: Vec<(CertificateSm2, u64)>,
}

impl Transaction {
    pub fn new(inputs: Vec<DigitalCurrencyWrapper>, outputs: Vec<(CertificateSm2, u64)>) -> Self {
        Self { inputs, outputs }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWrapper {
    msgtype: MsgType,
    /// 交易信息
    inner: Transaction,
    /// 付款方的签名集合
    signs: Vec<(CertificateSm2, SignatureSm2)>,
}

impl TransactionWrapper {
    pub fn new(inner: Transaction) -> Self {
        Self {
            msgtype: MsgType::Transaction,
            inner,
            signs: Vec::<(CertificateSm2, SignatureSm2)>::new(),
        }
    }

    pub fn get_inputs(&self) -> &Vec<DigitalCurrencyWrapper> {
        &self.inner.inputs
    }

    pub fn get_outputs(&self) -> &Vec<(CertificateSm2, u64)> {
        &self.inner.outputs
    }

    pub fn fill_sign(
        &mut self,
        keypair: &KeyPairSm2,
        rng: &mut impl RngCore,
    ) -> Result<(), CommStructError> {
        let inner_byte = self.inner.to_bytes();

        let signature = keypair
            .sign::<Sm3, _>(inner_byte.as_ref(), rng)
            .map_err(|_| CommStructError::SignatureError)?;

        self.signs.push((keypair.get_certificate(), signature));

        Ok(())
    }

    /// 判断交易体是否合法
    /// 判断依据
    ///     输入输出金额相等
    ///     输入有重复
    ///     输入每张货币所有者都对交易体有对应的签名
    /// 注： 单张货币合法性不在此判断，可由get_inputs取出另行判断
    pub fn check_validated(&self) -> bool {
        let mut collision = HashSet::<String>::new();
        let inner_byte = self.inner.to_bytes();

        let mut inputs_amount = 0;
        let mut outputs_amount = 0;
        for currency in &self.inner.inputs {
            inputs_amount += currency.get_body().get_amount();

            let curr_id = currency.get_body().get_id_str();
            if let Some(_) = collision.get(&curr_id) {
                return false;
            } else {
                collision.insert(curr_id);
            }

            let use_cert_sign = match self
                .signs
                .iter()
                .find(|&x| &x.0 == currency.get_body().get_owner())
            {
                Some(use_cert) => use_cert,
                None => return false,
            };
            if !use_cert_sign.0.verify::<Sm3>(&inner_byte, &use_cert_sign.1) {
                return false;
            }
        }
        for each in &self.inner.outputs {
            outputs_amount += each.1;
        }

        inputs_amount == outputs_amount
    }

    /// 生成交易后的货币
    ///     输入dcds的keypair
    ///     输出新生成的货币
    pub fn gen_new_currency(
        self,
        dcds_keypair: &KeyPairSm2,
        rng: &mut impl RngCore,
    ) -> Vec<DigitalCurrencyWrapper> {
        let mut ret = Vec::<DigitalCurrencyWrapper>::new();

        let cert_dcds = dcds_keypair.get_certificate();
        for each in &self.inner.outputs {
            let mut new_currency = DigitalCurrencyWrapper::new(
                MsgType::DigitalCurrency,
                DigitalCurrency::new(
                    each.0.clone(),
                    each.1.clone(),
                    cert_dcds.clone(),
                    Vec::<u8>::new(),
                    Vec::<u8>::new(),
                ),
            );

            new_currency.fill_kvhead(&dcds_keypair, rng).unwrap();

            ret.push(new_currency);
        }

        ret
    }
}

impl Bytes for Transaction {
    type BytesType = Vec<u8>;

    type Error = CommStructError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes).map_err(|_| CommStructError::DeSerializeError)
    }

    fn to_bytes(&self) -> Self::BytesType {
        bincode::serialize(self).expect("Transaction to_bytes exception")
    }
}

impl Bytes for TransactionWrapper {
    type BytesType = Vec<u8>;

    type Error = CommStructError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes).map_err(|_| CommStructError::DeSerializeError)
    }

    fn to_bytes(&self) -> Self::BytesType {
        bincode::serialize(self).expect("Transaction to_bytes exception")
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
