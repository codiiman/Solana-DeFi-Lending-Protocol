use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;

/// Repay borrowed assets
/// 
/// Users repay their borrows, reducing their debt and freeing up collateral.
#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.asset_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    /// User's token account (source of repayment)
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_token_account.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Reserve vault (destination for repaid assets)
    #[account(
        mut,
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig,
        constraint = reserve_vault.key() == market.reserve_vault @ LendingError::InvalidReserveVault
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// User's borrow position PDA
    #[account(
        mut,
        seeds = [b"borrow_position", market.key().as_ref(), user.key().as_ref()],
        bump,
        constraint = borrow_position.user == user.key() @ LendingError::Unauthorized,
        constraint = borrow_position.market == market.key() @ LendingError::InvalidMarketConfig,
        constraint = borrow_position.borrowed_amount > 0 @ LendingError::NoActiveBorrowPosition
    )]
    pub borrow_position: Account<'info, BorrowPosition>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Repay>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let market = &mut ctx.accounts.market;
    let position = &mut ctx.accounts.borrow_position;
    let clock = Clock::get()?;

    // Accrue interest before processing
    market.accrue_interest(&clock)?;

    // Calculate current debt
    let current_debt = position.calculate_debt(market)?;

    // Ensure repay amount doesn't exceed debt
    let repay_amount = amount.min(current_debt);
    require!(
        repay_amount <= current_debt,
        LendingError::RepayExceedsDebt
    );

    // Transfer assets from user to reserve vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.reserve_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, repay_amount)?;

    // Update borrow position
    let remaining_debt = current_debt
        .checked_sub(repay_amount)
        .ok_or(LendingError::MathOverflow)?;

    if remaining_debt == 0 {
        // Position fully repaid — reset for potential closure
        position.borrowed_amount = 0;
        position.cumulative_borrow_rate_snapshot = market.cumulative_borrow_rate;
        position.last_updated = clock.unix_timestamp;
    } else {
        // Partial repayment — update principal to remaining debt
        // and snapshot current rate so future interest accrues correctly
        position.borrowed_amount = remaining_debt;
        position.cumulative_borrow_rate_snapshot = market.cumulative_borrow_rate;
        position.last_updated = clock.unix_timestamp;
    }

    // Update market state
    market.total_borrowed = market.total_borrowed
        .checked_sub(repay_amount)
        .ok_or(LendingError::MathOverflow)?;

    emit!(Repaid {
        market: market.key(),
        user: ctx.accounts.user.key(),
        amount: repay_amount,
        total_borrowed: market.total_borrowed,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct Repaid {
    pub market: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub total_borrowed: u64,
    pub timestamp: i64,
}
