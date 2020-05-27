extern crate common_structure;
extern crate alloc;

use common_structure::issue_quota_request::IssueQuotaRequest;
use common_structure::quota_control_field::QuotaControlFieldWrapper;
use common_structure::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use alloc::vec::Vec;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    // 发行机构
    let keypair_dcds: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
        33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();
    let cert_dcds = keypair_dcds.get_certificate();

    // 钱包
    let wallet_keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        110, 79, 202, 249, 3, 215, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121,
        56, 137, 46, 25, 163, 13, 241, 160, 132, 203, 82, 177, 17,
    ])
    .unwrap();
    let wallet_cert = wallet_keypair_sm2.get_certificate();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((10, 5));
    issue_info.push((50, 2));
    issue_info.push((100, 1));

    let issue = IssueQuotaRequest::new(issue_info, cert_dcds);
    let quotas = issue.quota_distribution();

    assert_eq!(8, quotas.len());

    for (index, quota) in quotas.iter().enumerate() {
        let mut quota_control_field =
            QuotaControlFieldWrapper::new(MsgType::QuotaControlField, quota.clone());

        quota_control_field.fill_kvhead(&keypair_dcds, &mut rng).unwrap();

        let mut digital_currency = DigitalCurrencyWrapper::new(
            MsgType::DigitalCurrency,
            DigitalCurrency::new(quota_control_field, wallet_cert.clone()),
        );

        assert_eq!(
            match index {
                0 | 1 | 2 | 3 | 4 => 10,
                5 | 6 => 50,
                7 => 100,
                _ => panic!("error value"),
            },
            digital_currency
                .get_body()
                .get_quota_info()
                .get_body()
                .get_value()
        );

        digital_currency.fill_kvhead(&keypair_dcds, &mut rng).unwrap();
        let sign_bytes = digital_currency.to_bytes();

        let read_digitalcurrency = DigitalCurrencyWrapper::from_bytes(&sign_bytes).unwrap();
        assert_eq!(read_digitalcurrency.verfiy_kvhead().is_ok(), true);

        let serialized = serde_json::to_string(&read_digitalcurrency).unwrap();

        let deserialized: DigitalCurrencyWrapper = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            match index {
                0 | 1 | 2 | 3 | 4 => 10,
                5 | 6 => 50,
                7 => 100,
                _ => panic!("error value"),
            },
            deserialized
                .get_body()
                .get_quota_info()
                .get_body()
                .get_value()
        );
        assert_eq!(
            "\"03659AE6AFD520C54C48E58E96378B181ACD4CD14A096150281696F641A145864C\"",
            serde_json::to_string(&deserialized.get_body().get_wallet_cert()).unwrap()
        );
    }
}