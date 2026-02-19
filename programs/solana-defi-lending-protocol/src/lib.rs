use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod math;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("Lend1111111111111111111111111111111111");

#[program]
pub mod solana_defi_lending_protocol {
    use super::*;

    /// Initialize the global protocol configuration
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Create a new lending market
    pub fn create_market(
        ctx: Context<CreateMarket>,
        ltv_bps: u16,
        liquidation_threshold_bps: u16,
    ) -> Result<()> {
        instructions::market::handler(ctx, ltv_bps, liquidation_threshold_bps)
    }

    /// Supply assets to a lending market
    pub fn supply(ctx: Context<Supply>, amount: u64) -> Result<()> {
        instructions::supply::handler(ctx, amount)
    }

    /// Borrow assets from a lending market
    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        instructions::borrow::handler(ctx, amount)
    }

    /// Repay borrowed assets
    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        instructions::repay::handler(ctx, amount)
    }

    /// Withdraw supplied assets
    pub fn withdraw(ctx: Context<Withdraw>, supply_tokens: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, supply_tokens)
    }

    /// Liquidate an unhealthy borrow position
    pub fn liquidate(
        ctx: Context<Liquidate>,
        repay_amount: u64,
        min_collateral_amount: u64,
    ) -> Result<()> {
        instructions::liquidate::handler(ctx, repay_amount, min_collateral_amount)
    }

    /// Create a vault for automated yield strategies
    pub fn create_vault(
        ctx: Context<CreateVault>,
        strategy: u8,
        rebalance_threshold_bps: u16,
    ) -> Result<()> {
        instructions::vault::handler(ctx, strategy, rebalance_threshold_bps)
    }

    /// Rebalance vault allocations
    pub fn rebalance_vault(ctx: Context<RebalanceVault>) -> Result<()> {
        instructions::vault::rebalance_handler(ctx)
    }
}
}

// Re-export for external use
pub use state::*;
pub use errors::*;
pub use constants::*;
pub use math::*;
