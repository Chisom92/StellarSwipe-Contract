//! Leaderboard query functions for signal providers.
//!
//! Returns top providers ranked by success rate, total volume, or followers.
//! Rankings are computed on query from current stats (real-time updates).
//!
//! # Gas Costs
//! - get_leaderboard: O(P²) for P qualified providers (bubble sort)
//! - Typical: ~50-200k CPU units for 50 providers
//! - Leaderboard returns in <300ms (query uses current snapshot)

use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use crate::types::ProviderPerformance;

/// Minimum signals a provider must have to appear on the leaderboard
pub const MIN_SIGNALS_QUALIFICATION: u32 = 5;

/// Default number of providers returned
pub const DEFAULT_LEADERBOARD_LIMIT: u32 = 10;

/// Maximum number of providers that can be requested
pub const MAX_LEADERBOARD_LIMIT: u32 = 50;

/// Metric used to rank providers on the leaderboard
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaderboardMetric {
    SuccessRate,
    Volume,
    Followers, // Future feature - returns empty for MVP
}

/// Single entry in the leaderboard
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProviderLeaderboard {
    pub rank: u32,
    pub provider: Address,
    pub success_rate: u32,
    pub total_volume: i128,
    pub total_signals: u32,
}

/// Check if a provider qualifies for the leaderboard
#[inline]
fn is_qualified(stats: &ProviderPerformance) -> bool {
    stats.total_signals >= MIN_SIGNALS_QUALIFICATION && stats.success_rate > 0
}

/// Sort qualified vec by success rate (desc), tie-break by total_signals (desc)
fn sort_by_success_rate(qualified: &mut Vec<(Address, ProviderPerformance)>) {
    let len = qualified.len();
    if len <= 1 {
        return;
    }
    for _i in 0..len {
        let max_j = len - 1;
        for j in 0..max_j {
            let j_next = j + 1;
            let curr = qualified.get(j).unwrap();
            let next = qualified.get(j_next).unwrap();
            let swap = curr.1.success_rate < next.1.success_rate
                || (curr.1.success_rate == next.1.success_rate
                    && curr.1.total_signals < next.1.total_signals);
            if swap {
                qualified.set(j, next.clone());
                qualified.set(j_next, curr);
            }
        }
    }
}

/// Sort qualified vec by total volume (desc)
fn sort_by_volume(qualified: &mut Vec<(Address, ProviderPerformance)>) {
    let len = qualified.len();
    if len <= 1 {
        return;
    }
    for _i in 0..len {
        let max_j = len - 1;
        for j in 0..max_j {
            let j_next = j + 1;
            let curr = qualified.get(j).unwrap();
            let next = qualified.get(j_next).unwrap();
            if curr.1.total_volume < next.1.total_volume {
                qualified.set(j, next.clone());
                qualified.set(j_next, curr);
            }
        }
    }
}

/// Assign ranks (with tie handling: same rank, next rank skips) and build result.
/// For success rate: ties when success_rate and total_signals match.
/// For volume: ties when total_volume matches.
fn assign_ranks_and_build(
    env: &Env,
    sorted: &Vec<(Address, ProviderPerformance)>,
    limit: u32,
    by_success_rate: bool,
) -> Vec<ProviderLeaderboard> {
    let mut result = Vec::new(env);
    let take = limit.min(sorted.len());

    let mut rank: u32 = 1;

    for i in 0..take {
        let (provider, stats) = sorted.get(i).unwrap();
        let entry = ProviderLeaderboard {
            rank,
            provider: provider.clone(),
            success_rate: stats.success_rate,
            total_volume: stats.total_volume,
            total_signals: stats.total_signals,
        };
        result.push_back(entry);

        // Next rank: if next entry ties, keep same rank; else rank = i + 2
        let i_plus_1 = i + 1;
        if i_plus_1 < take {
            let curr = &sorted.get(i).unwrap().1;
            let next = &sorted.get(i_plus_1).unwrap().1;
            let tied = if by_success_rate {
                curr.success_rate == next.success_rate && curr.total_signals == next.total_signals
            } else {
                curr.total_volume == next.total_volume
            };
            if !tied {
                rank = i + 2;
            }
        }
    }

    result
}

/// Get the leaderboard for a given metric.
///
/// # Arguments
/// * `env` - Contract environment
/// * `stats_map` - Map of provider address to performance stats
/// * `metric` - Ranking metric (SuccessRate, Volume, or Followers)
/// * `limit` - Max providers to return (1-50, default 10 if 0)
///
/// # Returns
/// Top N qualified providers. Followers returns empty for MVP.
///
/// # Minimum qualification
/// - >= 5 signals submitted (terminal state)
/// - success_rate > 0 (exclude all-failed providers)
pub fn get_leaderboard(
    env: &Env,
    stats_map: &Map<Address, ProviderPerformance>,
    metric: LeaderboardMetric,
    limit: u32,
) -> Vec<ProviderLeaderboard> {
    // Followers: return empty for MVP
    if metric == LeaderboardMetric::Followers {
        return Vec::new(env);
    }

    // Clamp limit: default 10, max 50
    let limit = if limit == 0 {
        DEFAULT_LEADERBOARD_LIMIT
    } else if limit > MAX_LEADERBOARD_LIMIT {
        MAX_LEADERBOARD_LIMIT
    } else {
        limit
    };

    // Collect qualified providers (snapshot for consistency)
    let mut qualified: Vec<(Address, ProviderPerformance)> = Vec::new(env);
    for key in stats_map.keys() {
        if let Some(stats) = stats_map.get(key.clone()) {
            if is_qualified(&stats) {
                qualified.push_back((key, stats));
            }
        }
    }

    // Sort and build by metric
    match metric {
        LeaderboardMetric::SuccessRate => {
            sort_by_success_rate(&mut qualified);
            assign_ranks_and_build(env, &qualified, limit, true)
        }
        LeaderboardMetric::Volume => {
            sort_by_volume(&mut qualified);
            assign_ranks_and_build(env, &qualified, limit, false)
        }
        LeaderboardMetric::Followers => Vec::new(env), // Already handled above
    }
}
