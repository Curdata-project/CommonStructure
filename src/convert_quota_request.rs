use super::quota_control_field::{QuotaControlField, QuotaControlFieldWrapper};
use crate::QuotaError;
use asymmetric_crypto::hasher::sm3::Sm3;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::prelude::Local;
use dislog_hal::Bytes;
use dislog_hal::Hasher;
use kv_object::kv_object::{KVBody, KVObject};
use kv_object::prelude::{AttrProxy, KValueObject};
use kv_object::sm2::CertificateSm2;
use kv_object::KVObjectError;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertQoutaRequest {
    /// 转换请求ID，256bit，根据Hash[ 回收额度信息 | 目标发行信息 | 发行系统证书 | 时间戳 | 随机值 ]
    convert_id: [u8; 32],
    /// 需要回收的额度控制位，
    inputs: Vec<QuotaControlFieldWrapper>,
    /// 目标发行信息，要转换生成的额度信息，Vec<面值, 数目>，二元组根据面值从小到大排列，且以面值索引唯一
    outputs: Vec<(u64, u64)>,
    /// 发行系统的sm2证书
    delivery_system: CertificateSm2,
}

impl ConvertQoutaRequest {
    ///长度: 转换请求ID + 回收额度控制位(4 + Nu32 * QuotaControlField::QUOTA_LEN_WITH_KVHEAD)
    ///         + 目标发行信息(4 + Nu32 * 16) + 发行系统的sm2证书
    //pub const CONVERT_QUOTA_REQUEST_LEN: usize = 32 + 4 + N1 * QUOTA_LEN_WITH_KVHEAD + 4 + N2 * 16 + 33;
    pub const INPUTS_INFO_OFFSET: usize = 32;

    pub fn new(
        inputs: Vec<QuotaControlFieldWrapper>,
        outputs: Vec<(u64, u64)>,
        delivery_system: CertificateSm2,
    ) -> Self {
        let mut rng = rand::thread_rng();

        let mut hasher = Sm3::default();

        for each in inputs.iter() {
            hasher.update(&each.to_bytes());
        }

        for each in outputs.iter() {
            hasher.update(each.0.to_le_bytes());
            hasher.update(each.1.to_le_bytes());
        }

        hasher.update(delivery_system.to_bytes().as_ref());

        let now = Local::now();
        let timestamp = now.timestamp_millis();
        hasher.update(timestamp.to_le_bytes());

        let mut arr = [0u8; 32];
        rng.fill_bytes(&mut arr);
        hasher.update(arr);
        let convert_id = hasher.finalize();
        Self {
            convert_id,
            inputs,
            outputs,
            delivery_system,
        }
    }

    pub fn get_convert_id(&self) -> &[u8; 32] {
        &self.convert_id
    }

    pub fn get_inputs(&self) -> &Vec<QuotaControlFieldWrapper> {
        &self.inputs
    }

    pub fn get_outputs(&self) -> &Vec<(u64, u64)> {
        &self.outputs
    }

    pub fn get_delivery_system(&self) -> &CertificateSm2 {
        &self.delivery_system
    }
    /// 额度转换
    pub fn convert(&self) -> Result<Vec<QuotaControlField>, QuotaError> {
        let mut input_sum: u64 = 0;
        let mut output_sum: u64 = 0;

        // 检查转换前后额度是否一致
        for quota_control_field in &self.inputs {
            if quota_control_field.verfiy_kvhead().is_err() {
                return Err(QuotaError::QuotaConvertValidError);
            }

            let value = quota_control_field.get_body().get_value();
            input_sum += value;
        }

        for (value, amount) in &self.outputs {
            output_sum += value * amount;
        }

        if input_sum != output_sum {
            return Err(QuotaError::QuotaConvertSumError);
        }

        // 开始生成新额度控制位
        let mut ret = Vec::<QuotaControlField>::new();

        let mut rng = rand::thread_rng();

        let mut hasher = Sm3::default();
        hasher.update(&self.to_bytes()[..]);
        let trade_hash = hasher.finalize();
        for (value, amount) in self.outputs.iter() {
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

        Ok(ret)
    }
}

impl Bytes for ConvertQoutaRequest {
    type BytesType = Vec<u8>;

    type Error = KVObjectError;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 36 {
            return Err(KVObjectError::ValueValid);
        }
        let mut reads_len: usize = 0;

        // 读取convert_id
        let mut convert_id = [0u8; 32];
        convert_id.clone_from_slice(&bytes[0..ConvertQoutaRequest::INPUTS_INFO_OFFSET]);
        reads_len += 32;

        // 读取inputs
        let mut c = Cursor::new(&bytes[reads_len..]);
        let inputs_len = c.read_u32::<LittleEndian>().unwrap() as usize;

        if bytes.len() < reads_len + 4 + inputs_len * QuotaControlField::QUOTA_LEN_WITH_KVHEAD {
            return Err(KVObjectError::ValueValid);
        }

        let mut inputs = Vec::<QuotaControlFieldWrapper>::new();
        for i in 0..inputs_len {
            let offset = reads_len + 4 + i * QuotaControlField::QUOTA_LEN_WITH_KVHEAD;
            let end = reads_len + 4 + (i + 1) * QuotaControlField::QUOTA_LEN_WITH_KVHEAD;
            let quota_control_field = QuotaControlFieldWrapper::from_bytes(&bytes[offset..end])
                .map_err(|_| return KVObjectError::DeSerializeError)?;

            inputs.push(quota_control_field);
        }
        reads_len += 4 + inputs_len * QuotaControlField::QUOTA_LEN_WITH_KVHEAD;

        // 读取outputs
        let mut c = Cursor::new(&bytes[reads_len..]);
        let outputs_len = c.read_u32::<LittleEndian>().unwrap() as usize;

        if bytes.len() < reads_len + 4 + outputs_len * 16 {
            return Err(KVObjectError::ValueValid);
        }

        let mut outputs = Vec::<(u64, u64)>::new();
        for _ in 0..outputs_len {
            let value = c.read_u64::<LittleEndian>().unwrap();
            let amount = c.read_u64::<LittleEndian>().unwrap();
            outputs.push((value, amount));
        }
        reads_len += 4 + outputs_len * 16;

        // 读取delivery_system_
        if bytes.len() != reads_len + 33 {
            return Err(KVObjectError::ValueValid);
        }

        let mut delivery_system_ = [0u8; 33];
        delivery_system_.clone_from_slice(&bytes[reads_len..reads_len + 33]);
        let delivery_system = CertificateSm2::from_bytes(&delivery_system_[..])
            .map_err(|_| return KVObjectError::DeSerializeError)?;

        Ok(Self {
            convert_id,
            inputs,
            outputs,
            delivery_system,
        })
    }

    fn to_bytes(&self) -> Self::BytesType {
        let mut ret = Vec::<u8>::new();

        // 写入convert_id
        ret.extend_from_slice(&self.convert_id[..]);

        // 写入inputs
        ret.write_u32::<LittleEndian>(self.inputs.len() as u32)
            .unwrap();
        for each in self.inputs.iter() {
            ret.extend_from_slice(each.to_bytes().as_ref());
        }

        // 写入outputs
        ret.write_u32::<LittleEndian>(self.outputs.len() as u32)
            .unwrap();
        for each in self.outputs.iter() {
            ret.write_u64::<LittleEndian>(each.0).unwrap();
            ret.write_u64::<LittleEndian>(each.1).unwrap();
        }

        // 写入delivery_system
        ret.extend_from_slice(self.delivery_system.to_bytes().as_ref());

        ret
    }
}

impl AttrProxy for ConvertQoutaRequest {
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

impl KVBody for ConvertQoutaRequest {}

pub type ConvertQoutaRequestWrapper = KVObject<ConvertQoutaRequest>;

#[cfg(test)]
mod tests {

    #[test]
    fn test_convert_quota() {
        use super::super::issue_quota_request::IssueQuotaRequest;
        use super::super::quota_control_field::QuotaControlFieldWrapper;
        use super::{ConvertQoutaRequest, ConvertQoutaRequestWrapper};
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

        let mut issue_info = Vec::<(u64, u64)>::new();
        issue_info.push((10, 5));
        issue_info.push((50, 2));
        issue_info.push((100, 1));

        // 发行请求
        let issue = IssueQuotaRequest::new(issue_info, keypair_sm2.get_certificate());
        // 额度分发
        let quotas = issue.quota_distribution();
        let mut inputs = Vec::<QuotaControlFieldWrapper>::new();
        for each_quota in quotas.iter() {
            let mut quota_control_field =
                QuotaControlFieldWrapper::new(MsgType::QuotaControlField, each_quota.clone());

            quota_control_field.fill_kvhead(&keypair_sm2).unwrap();

            let sign_byte = quota_control_field.to_bytes();

            let read_quota = QuotaControlFieldWrapper::from_bytes(&sign_byte).unwrap();
            inputs.push(read_quota);
        }
        let mut outputs = Vec::<(u64, u64)>::new();
        outputs.push((50, 1));
        outputs.push((100, 2));
        let convert_request =
            ConvertQoutaRequest::new(inputs, outputs, keypair_sm2.get_certificate());

        let mut conver_wrapper =
            ConvertQoutaRequestWrapper::new(MsgType::ConvertQoutaRequest, convert_request.clone());

        conver_wrapper.fill_kvhead(&keypair_sm2).unwrap();

        let sign_bytes = conver_wrapper.to_bytes();

        let read_convert = ConvertQoutaRequestWrapper::from_bytes(sign_bytes.as_ref()).unwrap();

        assert_eq!(read_convert.verfiy_kvhead().is_ok(), true);

        let serialized = serde_json::to_string(&read_convert).unwrap();

        let deserialized: ConvertQoutaRequestWrapper = serde_json::from_str(&serialized).unwrap();

        assert_eq!(8, deserialized.get_body().get_inputs().len());
        for (index, quota) in deserialized.get_body().get_inputs().iter().enumerate() {
            assert_eq!(
                match index {
                    0 | 1 | 2 | 3 | 4 => 10,
                    5 | 6 => 50,
                    7 => 100,
                    _ => panic!("error value"),
                },
                quota.get_body().get_value()
            );
        }

        assert_eq!(2, deserialized.get_body().get_outputs().len());
        assert_eq!(
            &(50u64, 1u64),
            deserialized.get_body().get_outputs().get(0).unwrap()
        );
        assert_eq!(
            &(100u64, 2u64),
            deserialized.get_body().get_outputs().get(1).unwrap()
        );

        assert_eq!(
            serde_json::to_string(&conver_wrapper.get_body().get_delivery_system()).unwrap(),
            serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
        );

        // 额度转换
        let quotas = match convert_request.convert() {
            Ok(x) => x,
            Err(err) => panic!(err),
        };

        assert_eq!(3, quotas.len());

        for (index, quota) in quotas.iter().enumerate() {
            let mut quota_control_field =
                QuotaControlFieldWrapper::new(MsgType::QuotaControlField, quota.clone());

            assert_eq!(
                match index {
                    0 => 50,
                    1 | 2 => 100,
                    _ => panic!("error value"),
                },
                quota_control_field.get_body().get_value()
            );

            quota_control_field.fill_kvhead(&keypair_sm2).unwrap();
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
