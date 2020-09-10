use crate::digital_currency::DigitalCurrencyWrapper;
use crate::error::CommStructError;
use asymmetric_crypto::hasher::sm3::Sm3;
use asymmetric_crypto::prelude::Certificate;
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::prelude::AttrProxy;
use kv_object::sm2::{CertificateSm2, KeyPairSm2, SignatureSm2};
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionInner {
    /// 输入交易货币
    inputs: Vec<DigitalCurrencyWrapper>,
    /// 金额 （收款方证书, 收款金额）
    outputs: Vec<(CertificateSm2, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易信息
    inner: TransactionInner,
    /// 付款方的签名集合
    signs: Vec<(CertificateSm2, SignatureSm2)>,
}

impl Transaction {
    pub fn new(inputs: Vec<DigitalCurrencyWrapper>, outputs: Vec<(CertificateSm2, u64)>) -> Self {
        Self {
            inner: TransactionInner { inputs, outputs },
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

    pub fn check_sign(&self) -> bool {
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
                .find(|&x| x.0.to_bytes() == currency.get_body().get_owner().to_bytes())
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
}

impl Bytes for TransactionInner {
    type BytesType = Vec<u8>;

    type Error = CommStructError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes).map_err(|_| CommStructError::DeSerializeError)
    }

    fn to_bytes(&self) -> Self::BytesType {
        bincode::serialize(self).expect("Transaction to_bytes exception")
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
