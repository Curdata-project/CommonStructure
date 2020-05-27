extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use common_structure::convert_quota_request::{ConvertQoutaRequest, ConvertQoutaRequestWrapper};
use common_structure::issue_quota_request::IssueQuotaRequest;
use common_structure::quota_control_field::QuotaControlFieldWrapper;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241, 33,
        154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
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

        quota_control_field
            .fill_kvhead(&keypair_sm2, &mut rng)
            .unwrap();

        let sign_byte = quota_control_field.to_bytes();

        let read_quota = QuotaControlFieldWrapper::from_bytes(&sign_byte).unwrap();
        inputs.push(read_quota);
    }
    let mut outputs = Vec::<(u64, u64)>::new();
    outputs.push((50, 1));
    outputs.push((100, 2));
    let convert_request = ConvertQoutaRequest::new(inputs, outputs, keypair_sm2.get_certificate());

    let mut conver_wrapper =
        ConvertQoutaRequestWrapper::new(MsgType::ConvertQoutaRequest, convert_request.clone());

    conver_wrapper.fill_kvhead(&keypair_sm2, &mut rng).unwrap();

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
        Err(_) => panic!("error"),
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

        quota_control_field
            .fill_kvhead(&keypair_sm2, &mut rng)
            .unwrap();
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
            serde_json::to_string(&quota_control_field.get_body().get_delivery_system()).unwrap(),
            serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
        );
        assert_eq!(
            quota_control_field.get_body().get_trade_hash(),
            deserialized.get_body().get_trade_hash()
        );
    }
}
