use anchor_lang::prelude::*;

/// Maximum number of markets that can be created
pub const MAX_MARKETS: u8 = 50;

/// Maximum number of borrow positions per user
pub const MAX_BORROW_POSITIONS: u8 = 10;

/// Default loan-to-value ratio (75% = 7500 basis points)
pub const DEFAULT_LTV_BPS: u16 = 7500;

/// Default liquidation threshold (85% = 8500 basis points)
pub const DEFAULT_LIQUIDATION_THRESHOLD_BPS: u16 = 8500;

/// Minimum liquidation threshold (must be > LTV)
pub const MIN_LIQUIDATION_THRESHOLD_BPS: u16 = 8000;

/// Maximum LTV ratio (80% = 8000 basis points)
pub const MAX_LTV_BPS: u16 = 8000;

/// Protocol fee basis points (5% = 500 bps) on interest
pub const PROTOCOL_FEE_BPS: u16 = 500;

/// Liquidation bonus basis points (5% = 500 bps) - discount for liquidators
pub const LIQUIDATION_BONUS_BPS: u16 = 500;

/// Minimum health factor before liquidation (1.0 = 10000 basis points)
pub const MIN_HEALTH_FACTOR_BPS: u16 = 10000;

/// Interest rate model parameters
/// Base rate (2% APY = 0.02 / 365 / 24 / 3600 per second)
pub const BASE_RATE_PER_SECOND: u64 = 634_195_839; // ~2% APY

/// Optimal utilization rate (80% = 8000 basis points)
pub const OPTIMAL_UTILIZATION_BPS: u16 = 8000;

/// Slope 1: Interest rate slope below optimal utilization (10% APY per 10% utilization)
pub const SLOPE_1_PER_SECOND: u64 = 3_170_979_196; // ~10% APY

/// Slope 2: Interest rate slope above optimal utilization (100% APY per 10% utilization)
pub const SLOPE_2_PER_SECOND: u64 = 31_709_791_959; // ~100% APY

/// Scale factor for interest calculations (1e18 for precision)
pub const INTEREST_SCALE: u128 = 1_000_000_000_000_000_000;

/// Seconds per year (approximate)
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

/// Oracle price staleness threshold (5 minutes in seconds)
pub const ORACLE_STALENESS_THRESHOLD: i64 = 300;

/// Minimum borrow amount (0.01 tokens with 6 decimals)
pub const MIN_BORROW_AMOUNT: u64 = 10_000_000;

/// Minimum supply amount (0.1 tokens with 6 decimals)
pub const MIN_SUPPLY_AMOUNT: u64 = 100_000_000;

/// Vault strategy types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum VaultStrategy {
    /// Conservative: Low risk, stable yields
    Conservative = 0,
    /// Balanced: Moderate risk/reward
    Balanced = 1,
    /// Aggressive: Higher risk, higher potential yields
    Aggressive = 2,
}

/// Basis points (10000 = 100%)
pub const BPS_SCALE: u16 = 10000;
