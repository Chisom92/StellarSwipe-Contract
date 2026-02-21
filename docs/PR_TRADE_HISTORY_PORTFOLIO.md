# PR: Trade History & Portfolio for User Dashboard

## Summary

Adds trade history storage with paginated queries and portfolio calculation with P&L tracking for the auto-trade contract.

## Changes

### Trade History (`history.rs`)

- **Storage:** `Map<(Address, u64), HistoryTrade>` — user address + trade index
- **Struct:** `HistoryTrade` with id, signal_id, base_asset, amount, price, fee, timestamp, status
- **Status:** Pending, Executed, Failed, Cancelled
- **API:** `get_trade_history(user, offset, limit)` — newest first, default 20/page, max 100
- Per-user trade count maintained for indexing

### Portfolio (`portfolio.rs`)

- **API:** `get_portfolio(user)` → `Portfolio`
- **Portfolio:** assets, total_value_xlm, total_pnl
- **AssetHolding:** asset_id, amount, current_value_xlm, avg_entry_price, unrealized_pnl
- Prices from `risk::get_asset_price`; fallback to entry price if unavailable

### Wiring (`lib.rs`)

- `record_trade()` invoked on successful execution (Filled/PartiallyFilled)
- `get_trade_history`, `get_portfolio` exposed as contract functions
- `get_trade_history_legacy` retained for backward compatibility

## Definition of Done

- [x] All trades stored with complete details
- [x] Trade history query returns paginated results (newest first)
- [x] Portfolio calculation includes all user holdings
- [x] Unrealized P&L tracked per asset
- [x] Unit tests: `test_get_trade_history_paginated`, `test_get_trade_history_empty`, `test_get_portfolio`
- [x] Gas documentation in `contracts/auto_trade/GAS.md`

## Validation

- Execute 5+ trades, query history, verify chronological order
- Portfolio matches positions after trades
- Empty user returns empty history
- Pagination (offset/limit) works correctly
