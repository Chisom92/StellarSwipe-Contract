use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AutoTradeError {
    InvalidAmount,
    Unauthorized,
    SignalNotFound,
    SignalExpired,
    InsufficientBalance,
    InsufficientLiquidity,
}
