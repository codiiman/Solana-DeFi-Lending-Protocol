use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;

/// Create a new lending market
/// 
/// This instruction creates an isolated lending market for a specific asset.
/// Each market has its own configuration: LTV, liquidation threshold, oracle, etc.
#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// Asset mint for this market
    pub asset_mint: Account<'info, Mint>,

    /// Yield-bearing token mint (supply token)
    pub supply_mint: Account<'info, Mint>,

    /// Reserve vault (holds supplied assets)
    #[account(
        constraint = reserve_vault.mint == asset_mint.key() @ LendingError::InvalidMarketConfig
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// CHECK: Oracle account (Pyth or other price feed)
    pub oracle: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        space = Market::SIZE,
        seeds = [b"market", asset_mint.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateMarket>,
    ltv_bps: u16,
    liquidation_threshold_bps: u16,
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    require!(
        global_config.market_count < MAX_MARKETS,
        LendingError::InvalidMarketConfig
    );

    let market_id = global_config.market_count;
    let bump = ctx.bumps.get("market").copied().unwrap();

    market.initialize(
        market_id,
        ctx.accounts.asset_mint.key(),
        ctx.accounts.supply_mint.key(),
        ctx.accounts.reserve_vault.key(),
        ctx.accounts.oracle.key(),
        ltv_bps,
        liquidation_threshold_bps,
        ctx.accounts.creator.key(),
        bump,
        &clock,
    )?;

    global_config.market_count = global_config.market_count
        .checked_add(1)
        .ok_or(LendingError::MathOverflow)?;

    emit!(MarketCreated {
        market: market.key(),
        market_id,
        asset_mint: ctx.accounts.asset_mint.key(),
        ltv_bps,
        liquidation_threshold_bps,
        creator: ctx.accounts.creator.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub market_id: u8,
    pub asset_mint: Pubkey,
    pub ltv_bps: u16,
    pub liquidation_threshold_bps: u16,
    pub creator: Pubkey,
    pub timestamp: i64,
}
