use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use crate::constants::*;
use crate::math::*;

/// Global protocol configuration
#[account]
#[derive(Default)]
pub struct GlobalConfig {
    /// Protocol authority
    pub authority: Pubkey,
    /// Treasury PDA for protocol fees
    pub treasury: Pubkey,
    /// Protocol fee basis points
    pub protocol_fee_bps: u16,
    /// Total number of markets created
    pub market_count: u8,
    /// Bump seed for treasury PDA
    pub treasury_bump: u8,
    /// Reserved for future upgrades
    pub _reserved: [u8; 32],
}

impl GlobalConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // treasury
        2 +  // protocol_fee_bps
        1 +  // market_count
        1 +  // treasury_bump
        32;  // _reserved

    pub fn initialize(&mut self, authority: Pubkey, treasury: Pubkey, treasury_bump: u8) {
        self.authority = authority;
        self.treasury = treasury;
        self.protocol_fee_bps = PROTOCOL_FEE_BPS;
        self.market_count = 0;
        self.treasury_bump = treasury_bump;
    }
}

/// Lending market configuration
#[account]
pub struct Market {
    /// Market identifier (unique per asset)
    pub market_id: u8,
    /// Asset mint address
    pub asset_mint: Pubkey,
    /// Yield-bearing token mint (represents supply shares)
    pub supply_mint: Pubkey,
    /// Reserve vault (holds supplied assets)
    pub reserve_vault: Pubkey,
    /// Oracle account (Pyth or other price feed)
    pub oracle: Pubkey,
    /// Loan-to-value ratio in basis points (e.g., 7500 = 75%)
    pub ltv_bps: u16,
    /// Liquidation threshold in basis points (e.g., 8500 = 85%)
    pub liquidation_threshold_bps: u16,
    /// Total amount supplied (with accrued interest)
    pub total_supplied: u64,
    /// Total amount borrowed (with accrued interest)
    pub total_borrowed: u64,
    /// Total supply tokens minted (yield-bearing tokens)
    pub total_supply_tokens: u64,
    /// Last interest accrual timestamp
    pub last_accrual_timestamp: i64,
    /// Cumulative borrow rate (for interest accrual)
    pub cumulative_borrow_rate: u128,
    /// Cumulative supply rate (for interest accrual)
    pub cumulative_supply_rate: u128,
    /// Market paused flag
    pub paused: bool,
    /// Market creator
    pub creator: Pubkey,
    /// Timestamp when market was created
    pub created_at: i64,
    /// Bump seed for market PDA
    pub bump: u8,
}

impl Market {
    pub const SIZE: usize = 8 + // discriminator
        1 +  // market_id
        32 + // asset_mint
        32 + // supply_mint
        32 + // reserve_vault
        32 + // oracle
        2 +  // ltv_bps
        2 +  // liquidation_threshold_bps
        8 +  // total_supplied
        8 +  // total_borrowed
        8 +  // total_supply_tokens
        8 +  // last_accrual_timestamp
        16 + // cumulative_borrow_rate
        16 + // cumulative_supply_rate
        1 +  // paused
        32 + // creator
        8 +  // created_at
        1;   // bump

    pub fn initialize(
        &mut self,
        market_id: u8,
        asset_mint: Pubkey,
        supply_mint: Pubkey,
        reserve_vault: Pubkey,
        oracle: Pubkey,
        ltv_bps: u16,
        liquidation_threshold_bps: u16,
        creator: Pubkey,
        bump: u8,
        clock: &Clock,
    ) -> Result<()> {
        require!(
            liquidation_threshold_bps > ltv_bps,
            crate::errors::LendingError::LiquidationThresholdTooLow
        );
        require!(
            ltv_bps <= MAX_LTV_BPS,
            crate::errors::LendingError::InvalidLtvRatio
        );
        require!(
            liquidation_threshold_bps >= MIN_LIQUIDATION_THRESHOLD_BPS,
            crate::errors::LendingError::InvalidLiquidationThreshold
        );

        self.market_id = market_id;
        self.asset_mint = asset_mint;
        self.supply_mint = supply_mint;
        self.reserve_vault = reserve_vault;
        self.oracle = oracle;
        self.ltv_bps = ltv_bps;
        self.liquidation_threshold_bps = liquidation_threshold_bps;
        self.total_supplied = 0;
        self.total_borrowed = 0;
        self.total_supply_tokens = 0;
        self.last_accrual_timestamp = clock.unix_timestamp;
        self.cumulative_borrow_rate = INTEREST_SCALE;
        self.cumulative_supply_rate = INTEREST_SCALE;
        self.paused = false;
        self.creator = creator;
        self.created_at = clock.unix_timestamp;
        self.bump = bump;

        Ok(())
    }

    /// Accrue interest and update reserves
    pub fn accrue_interest(&mut self, clock: &Clock) -> Result<()> {
        if self.total_supplied == 0 && self.total_borrowed == 0 {
            self.last_accrual_timestamp = clock.unix_timestamp;
            return Ok(());
        }

        let seconds_elapsed = clock
            .unix_timestamp
            .checked_sub(self.last_accrual_timestamp)
            .ok_or(crate::errors::LendingError::MathOverflow)? as u64;

        if seconds_elapsed == 0 {
            return Ok(());
        }

        // Calculate utilization
        let utilization_bps = calculate_utilization_rate(self.total_borrowed, self.total_supplied)?;

        // Calculate borrow rate
        let borrow_rate_per_second = calculate_borrow_rate(utilization_bps)?;

        // Calculate supply rate
        let supply_rate_per_second = calculate_supply_rate(borrow_rate_per_second, utilization_bps)?;

        // Update cumulative rates
        self.cumulative_borrow_rate = (self.cumulative_borrow_rate as u128)
            .checked_mul(
                (INTEREST_SCALE + (borrow_rate_per_second as u128)
                    .checked_mul(seconds_elapsed as u128)
                    .ok_or(crate::errors::LendingError::MathOverflow)?
                    .checked_div(INTEREST_SCALE)
                    .ok_or(crate::errors::LendingError::MathOverflow)?) as u128,
            )
            .ok_or(crate::errors::LendingError::MathOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(crate::errors::LendingError::MathOverflow)?;

        self.cumulative_supply_rate = (self.cumulative_supply_rate as u128)
            .checked_mul(
                (INTEREST_SCALE + (supply_rate_per_second as u128)
                    .checked_mul(seconds_elapsed as u128)
                    .ok_or(crate::errors::LendingError::MathOverflow)?
                    .checked_div(INTEREST_SCALE)
                    .ok_or(crate::errors::LendingError::MathOverflow)?) as u128,
            )
            .ok_or(crate::errors::LendingError::MathOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(crate::errors::LendingError::MathOverflow)?;

        // Update total borrowed and supplied with accrued interest
        self.total_borrowed = (self.total_borrowed as u128)
            .checked_mul(self.cumulative_borrow_rate)
            .ok_or(crate::errors::LendingError::MathOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(crate::errors::LendingError::MathOverflow)? as u64;

        self.total_supplied = (self.total_supplied as u128)
            .checked_mul(self.cumulative_supply_rate)
            .ok_or(crate::errors::LendingError::MathOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(crate::errors::LendingError::MathOverflow)? as u64;

        self.last_accrual_timestamp = clock.unix_timestamp;

        Ok(())
    }
}

/// User's borrow position in a market
#[account]
pub struct BorrowPosition {
    /// User's wallet address
    pub user: Pubkey,
    /// Market this position is in
    pub market: Pubkey,
    /// Amount borrowed (principal)
    pub borrowed_amount: u64,
    /// Cumulative borrow rate when position was opened (for interest calculation)
    pub cumulative_borrow_rate_snapshot: u128,
    /// Timestamp when position was created
    pub created_at: i64,
    /// Last update timestamp
    pub last_updated: i64,
}

impl BorrowPosition {
    pub const SIZE: usize = 8 + // discriminator
        32 + // user
        32 + // market
        8 +  // borrowed_amount
        16 + // cumulative_borrow_rate_snapshot
        8 +  // created_at
        8;   // last_updated

    pub fn initialize(&mut self, user: Pubkey, market: Pubkey, borrowed_amount: u64, cumulative_rate: u128, clock: &Clock) {
        self.user = user;
        self.market = market;
        self.borrowed_amount = borrowed_amount;
        self.cumulative_borrow_rate_snapshot = cumulative_rate;
        self.created_at = clock.unix_timestamp;
        self.last_updated = clock.unix_timestamp;
    }

    /// Calculate current debt including accrued interest
    pub fn calculate_debt(&self, market: &Market) -> Result<u64> {
        let debt = (self.borrowed_amount as u128)
            .checked_mul(market.cumulative_borrow_rate)
            .ok_or(crate::errors::LendingError::MathOverflow)?
            .checked_div(self.cumulative_borrow_rate_snapshot)
            .ok_or(crate::errors::LendingError::MathOverflow)?;

        Ok(debt as u64)
    }
}

/// Vault account for automated yield strategies
#[account]
pub struct Vault {
    /// Vault owner/manager
    pub owner: Pubkey,
    /// Strategy type
    pub strategy: u8, // VaultStrategy enum as u8
    /// Total assets under management
    pub total_assets: u64,
    /// Market allocations (market pubkey -> allocation percentage in basis points)
    /// Stored as serialized Vec<(Pubkey, u16)>
    pub allocations: Vec<u8>, // Serialized allocations
    /// Last rebalance timestamp
    pub last_rebalance: i64,
    /// Rebalance threshold (basis points) - triggers rebalance when drift exceeds
    pub rebalance_threshold_bps: u16,
    /// Created timestamp
    pub created_at: i64,
    /// Bump seed
    pub bump: u8,
}

impl Vault {
    pub const BASE_SIZE: usize = 8 + // discriminator
        32 + // owner
        1 +  // strategy
        8 +  // total_assets
        4 +  // allocations vec length
        8 +  // last_rebalance
        2 +  // rebalance_threshold_bps
        8 +  // created_at
        1;   // bump

    pub fn initialize(
        &mut self,
        owner: Pubkey,
        strategy: u8,
        rebalance_threshold_bps: u16,
        bump: u8,
        clock: &Clock,
    ) {
        self.owner = owner;
        self.strategy = strategy;
        self.total_assets = 0;
        self.allocations = Vec::new();
        self.last_rebalance = clock.unix_timestamp;
        self.rebalance_threshold_bps = rebalance_threshold_bps;
        self.created_at = clock.unix_timestamp;
        self.bump = bump;
    }
}
