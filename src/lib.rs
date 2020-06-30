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

use rand::Error;
use rand::RngCore;
use getrandom::getrandom;

pub fn get_rng_core() -> impl RngCore + Send {
    GetRandomRng{}
}

#[derive(Default)]
struct GetRandomRng{}

impl RngCore for GetRandomRng {
    fn next_u32(&mut self) -> u32 {
        let mut tmp = [0u8; 4];
        getrandom(&mut tmp).unwrap();
        u32::from_le_bytes(tmp)
    }

    fn next_u64(&mut self) -> u64 {
        let mut tmp = [0u8; 8];
        getrandom(&mut tmp).unwrap();
        u64::from_le_bytes(tmp)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        getrandom(dest).unwrap();
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}