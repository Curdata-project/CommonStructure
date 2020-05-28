extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use common_structure::currency_convert_request::{
    CurrencyConvertRequest, CurrencyConvertRequestWrapper,
};
use common_structure::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
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

    // 钱包
    let wallet_keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        110, 79, 202, 249, 3, 215, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56,
        137, 46, 25, 163, 13, 241, 160, 132, 203, 82, 177, 17,
    ])
    .unwrap();
    let wallet_cert = wallet_keypair_sm2.get_certificate();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((10, 5));
    issue_info.push((50, 2));
    issue_info.push((100, 1));

    // 发行请求
    let issue = IssueQuotaRequest::new(issue_info, keypair_sm2.get_certificate());
    // 额度分发
    let quotas = issue.quota_distribution();
    let mut inputs = Vec::<DigitalCurrencyWrapper>::new();
    for each_quota in quotas.iter() {
        let mut quota_control_field =
            QuotaControlFieldWrapper::new(MsgType::QuotaControlField, each_quota.clone());

        quota_control_field
            .fill_kvhead(&keypair_sm2, &mut rng)
            .unwrap();

        let mut digital_currency = DigitalCurrencyWrapper::new(
            MsgType::DigitalCurrency,
            DigitalCurrency::new(quota_control_field, wallet_cert.clone()),
        );

        digital_currency
            .fill_kvhead(&keypair_sm2, &mut rng)
            .unwrap();

        inputs.push(digital_currency);
    }
    let mut outputs = Vec::<(u64, u64)>::new();
    outputs.push((50, 1));
    outputs.push((100, 2));
    let convert_request = CurrencyConvertRequest::new(inputs, outputs);

    let mut conver_wrapper =
        CurrencyConvertRequestWrapper::new(MsgType::ConvertQoutaRequest, convert_request.clone());

    conver_wrapper
        .fill_kvhead(&wallet_keypair_sm2, &mut rng)
        .unwrap();

    let sign_bytes = conver_wrapper.to_bytes();

    let read_convert = CurrencyConvertRequestWrapper::from_bytes(sign_bytes.as_ref()).unwrap();

    assert_eq!(read_convert.verfiy_kvhead().is_ok(), true);

    let serialized = serde_json::to_string(&read_convert).unwrap();

    let deserialized: CurrencyConvertRequestWrapper = serde_json::from_str(&serialized).unwrap();

    assert_eq!(8, deserialized.get_body().get_inputs().len());
    for (index, currency) in deserialized.get_body().get_inputs().iter().enumerate() {
        assert_eq!(
            match index {
                0 | 1 | 2 | 3 | 4 => 10,
                5 | 6 => 50,
                7 => 100,
                _ => panic!("error value"),
            },
            currency.get_body().get_quota_info().get_body().get_value()
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
}
