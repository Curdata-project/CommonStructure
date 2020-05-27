#![no_std]

extern crate alloc;

pub mod convert_quota_request;
pub mod digital_currency;
pub mod issue_quota_request;
pub mod quota_control_field;
pub mod quota_recycle_receipt;

pub enum QuotaError {
    QuotaConvertValidError,
    QuotaConvertSumError,
}
