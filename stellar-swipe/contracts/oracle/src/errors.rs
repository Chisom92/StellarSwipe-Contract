 feature/oracle-price-conversion
//! Oracle error types

=======
 main
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
 feature/oracle-price-conversion
    PriceNotFound = 1,
    NoConversionPath = 2,
    InvalidPath = 3,
    ConversionOverflow = 4,
    Unauthorized = 5,
    InvalidAsset = 6,
    StalePrice = 7,
=======
    Unauthorized = 1,
    OracleNotFound = 2,
    InvalidPrice = 3,
    OracleAlreadyExists = 4,
    InsufficientOracles = 5,
    LowReputation = 6,
 main
}
