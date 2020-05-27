extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use common_structure::issue_quota_request::IssueQuotaRequest;
use common_structure::quota_control_field::QuotaControlFieldWrapper;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    // 中心管理系统
    let keypair_cms: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241, 33,
        154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();

    // 货币发行系统
    let keypair_dcds: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241, 33,
        154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((10, 5));
    issue_info.push((50, 2));
    issue_info.push((100, 1));

    let issue_quota_request = IssueQuotaRequest::new(issue_info, keypair_dcds.get_certificate());
    let quotas = issue_quota_request.quota_distribution();

    assert_eq!(8, quotas.len());

    for (index, quota) in quotas.iter().enumerate() {
        let mut quota_control_field =
            QuotaControlFieldWrapper::new(MsgType::QuotaControlField, quota.clone());

        assert_eq!(
            match index {
                0 | 1 | 2 | 3 | 4 => 10,
                5 | 6 => 50,
                7 => 100,
                _ => panic!("error value"),
            },
            quota_control_field.get_body().get_value()
        );

        quota_control_field
            .fill_kvhead(&keypair_cms, &mut rng)
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
