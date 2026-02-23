use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
    Unauthorized = 1,
    OracleNotFound = 2,
    InvalidPrice = 3,
    OracleAlreadyExists = 4,
    InsufficientOracles = 5,
    LowReputation = 6,
}
