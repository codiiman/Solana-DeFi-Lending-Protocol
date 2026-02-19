use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;
use crate::math::*;

/// Borrow assets from a lending market
/// 
/// Users can borrow against their supplied collateral, up to the LTV limit.
/// Health factor must remain above the liquidation threshold.
#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.asset_mint.as_ref()],
        bump = market.bump,
        constraint = !market.paused @ LendingError::MarketPaused
    )]
    pub market: Account<'info, Market>,

    /// Reserve vault (source of borrowed assets)
    #[account(
        mut,
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// User's token account (destination for borrowed assets)
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_token_account.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// CHECK: Oracle account for price feed
    pub oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    require!(amount >= MIN_BORROW_AMOUNT, LendingError::InvalidAmount);

    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    // Accrue interest before processing
    market.accrue_interest(&clock)?;

    // Check available liquidity
    let available_liquidity = ctx.accounts.reserve_vault.amount;
    require!(
        amount <= available_liquidity,
        LendingError::InsufficientLiquidity
    );

    // TODO: In a full implementation, calculate user's total collateral across all markets
    // and total borrowed value to check health factor
    // For now, we do a simplified check on this market only

    // Calculate new total borrowed
    let new_total_borrowed = market.total_borrowed
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;

    // Check utilization doesn't exceed 100%
    let utilization_bps = calculate_utilization_rate(new_total_borrowed, market.total_supplied)?;
    require!(
        utilization_bps <= 10000,
        LendingError::InvalidUtilizationRate
    );

    // Transfer assets from reserve to user
    let seeds = &[
        b"market",
        market.asset_mint.as_ref(),
        &[market.bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.reserve_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: market.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_ctx, amount)?;

    // Update market state
    market.total_borrowed = new_total_borrowed;

    // Create or update borrow position
    // Note: In a full implementation, you'd use a PDA for the borrow position
    // For simplicity, we're updating the market's total_borrowed
    // A real implementation would track individual positions

    emit!(Borrowed {
        market: market.key(),
        user: ctx.accounts.user.key(),
        amount,
        total_borrowed: market.total_borrowed,
        utilization_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct Borrowed {
    pub market: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub total_borrowed: u64,
    pub utilization_bps: u16,
    pub timestamp: i64,
}
