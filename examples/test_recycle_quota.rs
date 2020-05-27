extern crate common_structure;
extern crate alloc;

use common_structure::issue_quota_request::IssueQuotaRequest;
use common_structure::quota_control_field::QuotaControlFieldWrapper;
use common_structure::quota_recycle_receipt::{QuotaRecycleReceipt, QuotaRecycleReceiptWrapper};
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use alloc::vec::Vec;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    let keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241,
        33, 154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();

    let mut recycle_info = Vec::<(u64, u64)>::new();
    recycle_info.push((10, 5));
    recycle_info.push((50, 2));
    recycle_info.push((100, 1));

    // 发行请求
    let issue = IssueQuotaRequest::new(recycle_info, keypair_sm2.get_certificate());
    // 额度分发
    let quotas = issue.quota_distribution();

    let mut need_recycles = Vec::<QuotaControlFieldWrapper>::new();
    for each_quota in quotas.iter() {
        let mut quota_control_field =
            QuotaControlFieldWrapper::new(MsgType::QuotaControlField, each_quota.clone());

        quota_control_field.fill_kvhead(&keypair_sm2, &mut rng).unwrap();

        let sign_byte = quota_control_field.to_bytes();

        let read_quota = QuotaControlFieldWrapper::from_bytes(&sign_byte).unwrap();
        need_recycles.push(read_quota);
    }
    let recycle_receipt =
        QuotaRecycleReceipt::recycle(&need_recycles, keypair_sm2.get_certificate()).unwrap();

    let mut recycle_receipt =
        QuotaRecycleReceiptWrapper::new(MsgType::QuotaRecycleReceipt, recycle_receipt);

    recycle_receipt.fill_kvhead(&keypair_sm2, &mut rng).unwrap();

    let sign_byte = recycle_receipt.to_bytes();

    let read_recycle_receipt = QuotaRecycleReceiptWrapper::from_bytes(&sign_byte).unwrap();

    assert_eq!(read_recycle_receipt.verfiy_kvhead().is_ok(), true);

    let serialized = serde_json::to_string(&read_recycle_receipt).unwrap();

    let deserialized: QuotaRecycleReceiptWrapper = serde_json::from_str(&serialized).unwrap();

    assert_eq!(
        recycle_receipt.get_body().get_recycle_id(),
        deserialized.get_body().get_recycle_id()
    );
    assert_eq!(
        serde_json::to_string(&recycle_receipt.get_body().get_delivery_system()).unwrap(),
        serde_json::to_string(deserialized.get_body().get_delivery_system()).unwrap()
    );

    assert_eq!(3, deserialized.get_body().get_recycle_info().len());
    assert_eq!(
        &(10u64, 5u64),
        deserialized.get_body().get_recycle_info().get(0).unwrap()
    );
    assert_eq!(
        &(50u64, 2u64),
        deserialized.get_body().get_recycle_info().get(1).unwrap()
    );
    assert_eq!(
        &(100u64, 1u64),
        deserialized.get_body().get_recycle_info().get(2).unwrap()
    );
}