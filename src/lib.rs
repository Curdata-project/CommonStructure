#![no_std]

extern crate alloc;

pub mod convert_quota_request;
pub mod currency_convert_request;
pub mod digital_currency;
pub mod issue_quota_request;
pub mod quota_control_field;
pub mod quota_recycle_receipt;
pub mod transaction;

pub enum QuotaError {
    QuotaConvertValidError,
    QuotaConvertSumError,
}

pub enum TransactionError {
    FundsOwnInvalid,
    SystemError,
}

use rand::RngCore;

pub fn get_rng_core() -> impl RngCore + Send {
    rand::rngs::OsRng
}
