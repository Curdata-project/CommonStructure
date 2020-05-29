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

    // 钱包，新的所有者
    let wallet_keypair_sm2_2: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        111, 215, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56, 79, 202, 249, 3,
        241, 160, 132, 203, 82, 177, 17, 137, 46, 25, 163, 13,
    ])
    .unwrap();
    let wallet_cert_2 = wallet_keypair_sm2_2.get_certificate();

    let mut issue_info = Vec::<(u64, u64)>::new();
    issue_info.push((10000, 1));
    issue_info.push((5000, 2));
    issue_info.push((1000, 5));

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
        Transaction::new(wallet_cert_2.clone(), pay_currency.get(0).unwrap().clone()),
    );

    // 支付者签名
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
    // dcds转移货币所有权
    let currency = deserialized
        .get_body()
        .trans_currency(&keypair_dcds)
        .unwrap();

    assert_eq!(
        10000,
        currency.get_body().get_quota_info().get_body().get_value()
    );

    // 转移前所有者
    assert_eq!(
        "\"03659AE6AFD520C54C48E58E96378B181ACD4CD14A096150281696F641A145864C\"",
        serde_json::to_string(
            deserialized
                .get_body()
                .get_currency()
                .get_body()
                .get_wallet_cert()
        )
        .unwrap()
    );

    assert_eq!(
        "\"0366AD51A3BF44EE15F4C8B278B0B695A3BFC2C56602CB647CDD77867A8AE92019\"",
        serde_json::to_string(deserialized.get_body().get_target()).unwrap()
    );

    // 当前所有者
    assert_eq!(
        "\"0366AD51A3BF44EE15F4C8B278B0B695A3BFC2C56602CB647CDD77867A8AE92019\"",
        serde_json::to_string(currency.get_body().get_wallet_cert()).unwrap()
    );

    assert_eq!(
        *CURRENCY_VALUE,
        [10000, 5000, 2000, 1000, 500, 200, 100, 50, 10]
    );
}
