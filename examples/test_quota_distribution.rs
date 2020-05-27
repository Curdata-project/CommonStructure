extern crate common_structure;
extern crate alloc;

use common_structure::issue_quota_request::{IssueQuotaRequest, IssueQuotaRequestWrapper};
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use alloc::vec::Vec;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    let keypair_cms: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
        33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((10, 5));
    issue_info.push((50, 2));
    issue_info.push((100, 1));
    let mut issue_quota = IssueQuotaRequestWrapper::new(
        MsgType::IssueQuotaRequest,
        IssueQuotaRequest::new(issue_info, keypair_cms.get_certificate()),
    );

    issue_quota.fill_kvhead(&keypair_cms, &mut rng).unwrap();

    let sign_bytes = issue_quota.to_bytes();

    let read_issue = IssueQuotaRequestWrapper::from_bytes(&sign_bytes).unwrap();

    assert_eq!(read_issue.verfiy_kvhead().is_ok(), true);

    let serialized = serde_json::to_string(&read_issue).unwrap();

    let deserialized: IssueQuotaRequestWrapper = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.verfiy_kvhead().is_ok(), true);
    assert_eq!(
        deserialized.get_body().get_issue_id(),
        issue_quota.get_body().get_issue_id()
    );
    assert_eq!(
        serde_json::to_string(&issue_quota.get_body().get_delivery_system()).unwrap(),
        serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
    );

    assert_eq!(3, deserialized.get_body().get_issue_info().len());
    assert_eq!(
        &(10u64, 5u64),
        deserialized.get_body().get_issue_info().get(0).unwrap()
    );
    assert_eq!(
        &(50u64, 2u64),
        deserialized.get_body().get_issue_info().get(1).unwrap()
    );
    assert_eq!(
        &(100u64, 1u64),
        deserialized.get_body().get_issue_info().get(2).unwrap()
    );
}