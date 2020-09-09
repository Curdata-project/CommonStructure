extern crate alloc;
extern crate common_structure;

use alloc::vec::Vec;
use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use kv_object::sm2::KeyPairSm2;
use hex::{FromHex, ToHex};
use rand::thread_rng;
use common_structure::digital_currency::{DigitalCurrency, DigitalCurrencyWrapper};

fn main(){
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

    let mut digital_currency = DigitalCurrencyWrapper::new(
        MsgType::DigitalCurrency,
        DigitalCurrency::new([0u8; 32], wallet_cert, 10000, cert_dcds, Vec::<u8>::new(), Vec::<u8>::new()),
    );

    digital_currency
            .fill_kvhead(&keypair_dcds, &mut rng)
            .unwrap();

    let bytes_currency = digital_currency.to_bytes().encode_hex_upper::<String>();

    let new_currency: DigitalCurrencyWrapper = DigitalCurrencyWrapper::from_bytes(&Vec::<u8>::from_hex(&bytes_currency).unwrap()).unwrap();

    assert_eq!(new_currency.verfiy_kvhead().is_ok(), true);
    assert_eq!(new_currency.to_bytes(), digital_currency.to_bytes());

    let str_currency = serde_json::to_string(&new_currency).unwrap();

    println!("serde_json {}", str_currency);

    let new_currency: DigitalCurrencyWrapper = serde_json::from_str(&str_currency).unwrap();

    assert_eq!(new_currency.verfiy_kvhead().is_ok(), true);
    assert_eq!(new_currency.to_bytes(), digital_currency.to_bytes());
}