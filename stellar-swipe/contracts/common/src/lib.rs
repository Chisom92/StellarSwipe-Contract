#![no_std]

pub mod assets;
pub mod emergency;
pub mod rate_limit;
pub mod replay_protection;

pub use assets::{validate_asset_pair, Asset, AssetPair, AssetPairError};
pub use emergency::{PauseState, CAT_TRADING, CAT_SIGNALS, CAT_STAKES, CAT_ALL};
pub use rate_limit::{ActionType, RateLimitConfig, check_rate_limit, record_action, set_config as set_rate_limit_config};
pub use replay_protection::{ReplayError, current_nonce, verify_and_commit};
