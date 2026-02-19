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
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Repay>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    // Accrue interest before processing
    market.accrue_interest(&clock)?;

    // Transfer assets from user to reserve vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.reserve_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Update market state
    // Note: In a full implementation, you'd update the specific borrow position
    // For now, we're simplifying by updating total_borrowed
    market.total_borrowed = market.total_borrowed
        .checked_sub(amount)
        .ok_or(LendingError::MathOverflow)?;

    emit!(Repaid {
        market: market.key(),
        user: ctx.accounts.user.key(),
        amount,
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
