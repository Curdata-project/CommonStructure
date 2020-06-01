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

use alloc::vec;
use alloc::vec::Vec;
use lazy_static::*;

lazy_static! {
    /// 100元，50元，20元，10元，5元，2元，1元，5角，1角
    pub static ref CURRENCY_VALUE: Vec::<u64> = vec![10000, 5000, 2000, 1000, 500, 200, 100, 50, 10];
}

use rand::RngCore;
pub fn get_rng_core() -> impl RngCore {
    rand::thread_rng()
}
