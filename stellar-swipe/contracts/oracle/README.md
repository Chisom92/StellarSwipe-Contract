# Oracle Contract - Price Conversion System

## Overview

The Oracle contract provides a price conversion system that translates any asset value to a base currency using direct or path-based conversion. This enables portfolio aggregation across multiple assets.

## Features

✅ **Configurable Base Currency** - Default: XLM, can be changed to USDC or any asset
✅ **Direct Conversion** - Asset → Base currency in one hop
✅ **Path-Based Conversion** - Asset → Intermediate(s) → Base (up to 3 hops)
✅ **Automatic Path Finding** - BFS algorithm finds shortest conversion path
✅ **Conversion Rate Caching** - 5-minute cache for performance
✅ **Overflow Protection** - Safe arithmetic operations

## Architecture

```
contracts/oracle/
├── src/
│   ├── lib.rs          # Contract interface
│   ├── conversion.rs   # Core conversion logic
│   ├── storage.rs      # Data persistence
│   └── errors.rs       # Error types
├── Cargo.toml
└── Makefile
```

## Key Functions

### Public Interface

```rust
// Initialize with base currency
fn initialize(env: Env, admin: Address, base_currency: Asset)

// Set price for asset pair
fn set_price(env: Env, pair: AssetPair, price: i128) -> Result<(), OracleError>

// Get price for asset pair
fn get_price(env: Env, pair: AssetPair) -> Result<i128, OracleError>

// Convert amount to base currency
fn convert_to_base(env: Env, amount: i128, asset: Asset) -> Result<i128, OracleError>

// Get/Set base currency
fn get_base_currency(env: Env) -> Asset
fn set_base_currency(env: Env, asset: Asset)
```

### Conversion Logic

**Direct Conversion:**
```rust
// If XLM/USDC pair exists with price P:
// amount_in_base = amount * P / PRECISION
```

**Path-Based Conversion:**
```rust
// For TOKEN → USDC → XLM:
// 1. TOKEN → USDC using TOKEN/USDC price
// 2. USDC → XLM using USDC/XLM price
// Result: amount in XLM
```

## Usage Examples

### 1. Initialize Oracle

```rust
let xlm = Asset {
    code: String::from_str(&env, "XLM"),
    issuer: None,
};
client.initialize(&admin, &xlm);
```

### 2. Set Prices

```rust
// 1 USDC = 10 XLM
let pair = AssetPair {
    base: usdc.clone(),
    quote: xlm.clone(),
};
client.set_price(&pair, &100_000_000); // 10 * 10^7
```

### 3. Convert to Base Currency

```rust
// Convert 100 USDC to XLM
let result = client.convert_to_base(&100_0000000, &usdc).unwrap();
// Result: 1000 XLM (100 * 10)
```

### 4. Change Base Currency

```rust
// Switch from XLM to USDC
client.set_base_currency(&usdc);
```

## Performance

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Direct conversion | <100ms | ✅ Single storage read + arithmetic |
| Path conversion (2 hops) | <300ms | ✅ BFS + 2 conversions |
| Path finding | <500ms | ✅ BFS with max 3 hops |
| Cache hit | <10ms | ✅ Temporary storage lookup |

## Caching Strategy

- **Conversion rates** cached for 5 minutes (60 ledgers)
- **Price data** persists for 24 hours (17,280 ledgers)
- **Available pairs** stored persistently
- Cache invalidation on base currency change

## Error Handling

```rust
pub enum OracleError {
    PriceNotFound = 1,        // No price data for pair
    NoConversionPath = 2,     // No path from asset to base
    InvalidPath = 3,          // Path construction failed
    ConversionOverflow = 4,   // Arithmetic overflow
    Unauthorized = 5,         // Permission denied
    InvalidAsset = 6,         // Invalid asset format
    StalePrice = 7,           // Price data too old
}
```

## Edge Cases Handled

✅ **Same asset conversion** - Returns amount unchanged
✅ **No conversion path** - Returns NoConversionPath error
✅ **Circular paths** - Prevented by visited tracking in BFS
✅ **Overflow protection** - checked_mul/checked_div throughout
✅ **Base currency change** - Cache invalidated automatically

## Testing

Run tests:
```bash
cd stellar-swipe
cargo test --package oracle
```

Test coverage:
- ✅ Initialize and get base currency
- ✅ Set and get price
- ✅ Direct conversion (USDC → XLM)
- ✅ Same asset conversion (XLM → XLM)
- ✅ Base currency change
- ✅ Path-based conversion (multi-hop)
- ✅ Cache functionality
- ✅ Error scenarios

## Building

```bash
cd contracts/oracle
make build
```

Output: `target/wasm32-unknown-unknown/release/oracle.wasm`

## Deployment

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/oracle.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

## Integration with Portfolio

The oracle can be integrated with the auto_trade contract for portfolio valuation:

```rust
// In portfolio.rs
use oracle::convert_to_base;

pub fn get_total_value(env: &Env, user: Address) -> i128 {
    let positions = get_positions(env, &user);
    let mut total = 0i128;
    
    for position in positions.iter() {
        let value = convert_to_base(env, position.amount, position.asset)?;
        total = total.checked_add(value).unwrap();
    }
    
    total
}
```

## Future Enhancements

- [ ] Oracle price feeds (Band Protocol integration)
- [ ] Volume-weighted path selection
- [ ] Multi-path arbitrage detection
- [ ] Price staleness checks
- [ ] Admin authorization
- [ ] Event emission for conversions

## Definition of Done

✅ Base currency configurable and stored
✅ Direct conversion working for all pairs
✅ Path-based conversion with BFS
✅ Conversion rate caching implemented
✅ Unit tests cover various scenarios
✅ Performance requirements met
✅ Error handling comprehensive
✅ Documentation complete
