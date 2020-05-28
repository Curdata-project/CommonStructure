use super::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use alloc::vec::Vec;
use byteorder::{ByteOrder, LittleEndian};
use dislog_hal::Bytes;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::AttrProxy;
use kv_object::KVObjectError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConvertRequest {
    /// 需要拆分的额度控制位，
    inputs: Vec<DigitalCurrencyWrapper>,
    /// 目标兑换信息，要兑换生成的额度信息，Vec<面值, 数目>，二元组根据面值从小到大排列，且以面值索引唯一
    outputs: Vec<(u64, u64)>,
}

impl CurrencyConvertRequest {
    ///长度: 拆分额度控制位(4 + Nu32 * QuotaControlField::QUOTA_LEN_WITH_KVHEAD)
    ///         + 目标兑换信息(4 + Nu32 * 16)
    //pub const CONVERT_QUOTA_REQUEST_LEN: usize = 4 + N1 * QUOTA_LEN_WITH_KVHEAD + 4 + N2 * 16;

    pub fn new(inputs: Vec<DigitalCurrencyWrapper>, outputs: Vec<(u64, u64)>) -> Self {
        Self { inputs, outputs }
    }

    pub fn get_inputs(&self) -> &Vec<DigitalCurrencyWrapper> {
        &self.inputs
    }

    pub fn get_outputs(&self) -> &Vec<(u64, u64)> {
        &self.outputs
    }
}

impl Bytes for CurrencyConvertRequest {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 4 {
            return Err(KVObjectError::ValueValid);
        }
        let mut reads_len: usize = 0;

        // 读取inputs
        let inputs_len = LittleEndian::read_u32(&bytes[reads_len..reads_len + 4]) as usize;

        if bytes.len() < reads_len + 4 + inputs_len * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD {
            return Err(KVObjectError::ValueValid);
        }

        let mut inputs = Vec::<DigitalCurrencyWrapper>::new();
        for i in 0..inputs_len {
            let offset = reads_len + 4 + i * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD;
            let end = reads_len + 4 + (i + 1) * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD;
            let quota_control_field = DigitalCurrencyWrapper::from_bytes(&bytes[offset..end])
                .map_err(|_| return KVObjectError::DeSerializeError)?;

            inputs.push(quota_control_field);
        }
        reads_len += 4 + inputs_len * DigitalCurrency::CURRENCY_LEN_WITH_KVHEAD;

        // 读取outputs
        let outputs_len = LittleEndian::read_u32(&bytes[reads_len..reads_len + 4]) as usize;

        if bytes.len() < reads_len + 4 + outputs_len * 16 {
            return Err(KVObjectError::ValueValid);
        }

        let mut outputs = Vec::<(u64, u64)>::new();
        for i in 0..outputs_len {
            let offset = reads_len + 4 + i * 16;
            let value = LittleEndian::read_u64(&bytes[offset..offset + 8]);
            let amount = LittleEndian::read_u64(&bytes[offset + 8..offset + 16]);

            outputs.push((value, amount));
        }

        Ok(Self { inputs, outputs })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();
        let mut buf_32 = [0; 4];
        let mut buf_64 = [0; 8];

        // 写入inputs
        LittleEndian::write_u32(&mut buf_32, self.inputs.len() as u32);
        ret.extend_from_slice(&buf_32);
        for each in self.inputs.iter() {
            ret.extend_from_slice(each.to_bytes().as_ref());
        }

        // 写入outputs
        LittleEndian::write_u32(&mut buf_32, self.outputs.len() as u32);
        ret.extend_from_slice(&buf_32);
        for each in self.outputs.iter() {
            LittleEndian::write_u64(&mut buf_64, each.0);
            ret.extend_from_slice(&buf_64);
            LittleEndian::write_u64(&mut buf_64, each.1);
            ret.extend_from_slice(&buf_64);
        }

        ret
    }
}

impl AttrProxy for CurrencyConvertRequest {
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

impl KVBody for CurrencyConvertRequest {}

pub type CurrencyConvertRequestWrapper = KVObject<CurrencyConvertRequest>;
