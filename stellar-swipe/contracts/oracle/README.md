# Oracle Reputation & Automatic Weight Adjustment

This Soroban smart contract implements an oracle reputation system that tracks oracle accuracy and automatically adjusts weights to favor better-performing oracles.

## Features

### 1. Oracle Reputation Tracking
- Tracks total submissions and accurate submissions per oracle
- Calculates average deviation from consensus
- Maintains reputation score (0-100)
- Records last slash timestamp for consistency scoring

### 2. Reputation Calculation
Reputation is calculated using three components:
- **60% Accuracy Rate**: Based on submissions within acceptable deviation
- **30% Deviation Score**: Lower average deviation = higher score
- **10% Consistency Score**: No slashes in the past 7 days = bonus points

### 3. Automatic Weight Adjustment
Weights are automatically adjusted based on reputation:
- **90-100**: Weight 10 (High reputation)
- **75-89**: Weight 5 (Good reputation)
- **60-74**: Weight 2 (Average reputation)
- **50-59**: Weight 1 (Below average)
- **<50**: Weight 0 (Removed)

### 4. Accuracy Tracking
After consensus is established, each oracle's submission is evaluated:
- **Accurate**: Within 1% of consensus
- **Moderately Accurate**: Within 5% of consensus
- **Inaccurate**: >5% deviation

### 5. Slashing Mechanism
Oracles are penalized for:
- **Major Deviation (>20%)**: -20 reputation points
- **Signature Verification Failure**: -30 reputation points

### 6. Oracle Removal
Oracles are removed if:
- Reputation score falls below 50
- Accuracy rate <50% over 100+ submissions
- System maintains minimum of 2 oracles

## Contract Functions

### Admin Functions
- `initialize(admin: Address)` - Initialize contract
- `register_oracle(admin: Address, oracle: Address)` - Register new oracle
- `remove_oracle(admin: Address, oracle: Address)` - Manually remove oracle

### Oracle Functions
- `submit_price(oracle: Address, price: i128)` - Submit price data
- `calculate_consensus()` - Calculate consensus and update reputations

### Query Functions
- `get_oracle_reputation(oracle: Address)` - Get oracle stats
- `get_oracles()` - Get all registered oracles
- `get_consensus_price()` - Get latest consensus price

## Data Structures

### OracleReputation
```rust
pub struct OracleReputation {
    pub total_submissions: u32,
    pub accurate_submissions: u32,
    pub avg_deviation: i128,
    pub reputation_score: u32,
    pub weight: u32,
    pub last_slash: u64,
}
```

### ConsensusPriceData
```rust
pub struct ConsensusPriceData {
    pub price: i128,
    pub timestamp: u64,
    pub num_oracles: u32,
}
```

## Events

The contract emits the following events:
- `oracle_removed` - When an oracle is removed
- `weight_adjusted` - When oracle weight changes
- `oracle_slashed` - When an oracle is penalized
- `price_submitted` - When a price is submitted
- `consensus_reached` - When consensus is calculated

## Building

```bash
make build
```

## Testing

```bash
make test
```

## Usage Example

```rust
// Initialize contract
client.initialize(&admin);

// Register oracles
client.register_oracle(&admin, &oracle1);
client.register_oracle(&admin, &oracle2);
client.register_oracle(&admin, &oracle3);

// Oracles submit prices
client.submit_price(&oracle1, &100_000_000);
client.submit_price(&oracle2, &101_000_000);
client.submit_price(&oracle3, &99_000_000);

// Calculate consensus (automatically updates reputations)
let consensus = client.calculate_consensus();

// Check oracle reputation
let reputation = client.get_oracle_reputation(&oracle1);
println!("Reputation: {}", reputation.reputation_score);
println!("Weight: {}", reputation.weight);
```

## Edge Cases Handled

1. **New Oracle**: Starts with default weight of 1 and reputation of 50
2. **Reputation Recovery**: Oracles can improve reputation through accurate submissions
3. **Minimum Oracles**: System maintains at least 2 oracles even if all perform poorly
4. **Oracle Manipulation**: Sudden reputation drops are detected via slashing mechanism

## Validation Tests

All validation scenarios from the requirements are covered:
- ✅ Submit prices from 3 oracles (1 accurate, 1 moderate, 1 poor)
- ✅ Run reputation calculation, verify scores
- ✅ Verify weights adjusted correctly
- ✅ Submit consistently bad data, verify oracle removal
- ✅ Test reputation recovery after improvement

## Definition of Done

- ✅ Oracle accuracy tracked per submission
- ✅ Reputation calculated from accuracy + deviation
- ✅ Weights adjusted automatically based on reputation
- ✅ Slashing implemented for poor performance
- ✅ Unit tests verify reputation logic
- ✅ Events emitted on weight changes
