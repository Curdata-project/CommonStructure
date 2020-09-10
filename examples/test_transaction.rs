extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use common_structure::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};
use common_structure::get_rng_core;
use common_structure::transaction::Transaction;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
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

    // 钱包A
    let wallet_keypair_a: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        110, 79, 202, 249, 3, 215, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56,
        137, 46, 25, 163, 13, 241, 160, 132, 203, 82, 177, 17,
    ])
    .unwrap();
    let wallet_cert_a = wallet_keypair_a.get_certificate();

    // 钱包B
    let wallet_keypair_b: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        46, 25, 163, 13, 241, 160, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56,
        137, 132, 203, 82, 110, 79, 202, 249, 3, 215, 177, 17,
    ])
    .unwrap();
    let wallet_cert_b = wallet_keypair_b.get_certificate();

    // 钱包C
    let wallet_keypair_c: KeyPairSm2 = KeyPairSm2::generate_from_seed([
        203, 82, 110, 13, 241, 160, 135, 141, 4, 220, 33, 154, 195, 196, 125, 33, 85, 57, 121, 56,
        137, 132, 46, 25, 163, 79, 202, 249, 3, 215, 177, 17,
    ])
    .unwrap();
    let wallet_cert_c = wallet_keypair_c.get_certificate();

    let mut currency_1 = DigitalCurrencyWrapper::new(
        MsgType::DigitalCurrency,
        DigitalCurrency::new(
            wallet_cert_a.clone(),
            10000,
            cert_dcds.clone(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
        ),
    );

    currency_1.fill_kvhead(&keypair_dcds, &mut rng).unwrap();

    let mut currency_2 = DigitalCurrencyWrapper::new(
        MsgType::DigitalCurrency,
        DigitalCurrency::new(
            wallet_cert_b.clone(),
            10000,
            cert_dcds,
            Vec::<u8>::new(),
            Vec::<u8>::new(),
        ),
    );

    currency_2.fill_kvhead(&keypair_dcds, &mut rng).unwrap();

    let mut inputs = Vec::<DigitalCurrencyWrapper>::new();
    inputs.push(currency_1);
    inputs.push(currency_2);
    let mut outputs = Vec::<(CertificateSm2, u64)>::new();
    outputs.push((wallet_cert_c.clone(), 20000));
    let mut transaction = Transaction::new(inputs, outputs);

    let mut rng = get_rng_core();
    transaction.fill_sign(&wallet_keypair_b, &mut rng).unwrap();
    transaction.fill_sign(&wallet_keypair_a, &mut rng).unwrap();

    assert_eq!(transaction.check_validated(), true);
    assert_eq!(transaction.get_inputs().len(), 2);
    assert_eq!(transaction.get_inputs()[0].get_body().get_amount(), 10000);
    assert_eq!(
        transaction.get_inputs()[0].get_body().get_owner(),
        &wallet_cert_a
    );
    assert_eq!(transaction.get_inputs()[1].get_body().get_amount(), 10000);
    assert_eq!(
        transaction.get_inputs()[1].get_body().get_owner(),
        &wallet_cert_b
    );
    assert_eq!(transaction.get_outputs().len(), 1);
    assert_eq!(transaction.get_outputs()[0].0, wallet_cert_c);
    assert_eq!(transaction.get_outputs()[0].1, 20000);

    let new_currencys = transaction.gen_new_currency(&keypair_dcds, &mut rng);
    assert_eq!(new_currencys.len(), 1);
    assert_eq!(new_currencys[0].get_body().get_amount(), 20000);
    assert_eq!(new_currencys[0].get_body().get_owner(), &wallet_cert_c);
}
