//! Oracle error types

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
    PriceNotFound = 1,
    NoConversionPath = 2,
    InvalidPath = 3,
    ConversionOverflow = 4,
    Unauthorized = 5,
    InvalidAsset = 6,
    StalePrice = 7,
}
