extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use common_structure::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use common_structure::issue_quota_request::IssueQuotaRequest;
use common_structure::quota_control_field::QuotaControlFieldWrapper;
use common_structure::transaction::{Transaction, TransactionWrapper};
use common_structure::CURRENCY_VALUE;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();

    // 发行机构
    let keypair_dcds: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        3, 215, 135, 141, 4, 220, 160, 132, 203, 82, 177, 17, 56, 137, 46, 25, 163, 13, 241, 33,
        154, 195, 196, 125, 33, 85, 57, 121, 110, 79, 202, 249,
    ])
    .unwrap();
    let cert_dcds = keypair_dcds.get_certificate();

    // 钱包
    let wallet_keypair_sm2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        110, 79, 202, 249, 3, 215, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56,
        137, 46, 25, 163, 13, 241, 160, 132, 203, 82, 177, 17,
    ])
    .unwrap();
    let wallet_cert = wallet_keypair_sm2.get_certificate();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((1000, 5));
    issue_info.push((5000, 2));
    issue_info.push((10000, 1));

    let issue = IssueQuotaRequest::new(issue_info, cert_dcds);
    let quotas = issue.quota_distribution();

    assert_eq!(8, quotas.len());

    // 组建支付货币列表
    let mut pay_currency = Vec::<DigitalCurrencyWrapper>::new();
    for (_, quota) in quotas.iter().enumerate() {
        let mut quota_control_field =
            QuotaControlFieldWrapper::new(MsgType::QuotaControlField, quota.clone());

        quota_control_field
            .fill_kvhead(&keypair_dcds, &mut rng)
            .unwrap();

        let mut digital_currency = DigitalCurrencyWrapper::new(
            MsgType::DigitalCurrency,
            DigitalCurrency::new(quota_control_field, wallet_cert.clone()),
        );

        digital_currency
            .fill_kvhead(&keypair_dcds, &mut rng)
            .unwrap();

        pay_currency.push(digital_currency);
    }

    let mut transaction_ok_1 = TransactionWrapper::new(
        MsgType::Transaction,
        Transaction::new(wallet_cert.clone(), pay_currency.clone()),
    );

    transaction_ok_1
        .fill_kvhead(&wallet_keypair_sm2, &mut rng)
        .unwrap();
    let sign_bytes = transaction_ok_1.to_bytes();

    println!("{:?}", sign_bytes.len());

    let read_transaction_ok_1 = TransactionWrapper::from_bytes(&sign_bytes).unwrap();
    assert_eq!(read_transaction_ok_1.verfiy_kvhead().is_ok(), true);

    let serialized = serde_json::to_string(&read_transaction_ok_1).unwrap();

    let deserialized: TransactionWrapper = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        transaction_ok_1.get_body().get_txid(),
        deserialized.get_body().get_txid()
    );
    assert_eq!(
        serde_json::to_string(&transaction_ok_1.get_body().get_target()).unwrap(),
        serde_json::to_string(deserialized.get_body().get_target()).unwrap()
    );

    let currencys = deserialized.get_body().get_currencys();

    assert_eq!(8, currencys.len());

    for (index, currency) in currencys.iter().enumerate() {
        assert_eq!(
            match index {
                0 | 1 | 2 | 3 | 4 => 1000,
                5 | 6 => 5000,
                7 => 10000,
                _ => panic!("error value"),
            },
            currency.get_body().get_quota_info().get_body().get_value()
        );
    }

    assert_eq!(
        *CURRENCY_VALUE,
        [10000, 5000, 2000, 1000, 500, 200, 100, 50, 10]
    );
}
