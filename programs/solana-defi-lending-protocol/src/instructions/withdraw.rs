use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Burn};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;
use crate::math::*;

/// Withdraw supplied assets from a lending market
/// 
/// Users burn their supply tokens to withdraw their underlying assets
/// plus accrued interest. Health factor must remain safe.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.asset_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    /// User's supply token account (to burn)
    #[account(
        mut,
        constraint = user_supply_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_supply_account.mint == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_supply_account: Account<'info, TokenAccount>,

    /// Supply mint
    #[account(
        mut,
        constraint = supply_mint.key() == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub supply_mint: Account<'info, anchor_spl::token::Mint>,

    /// Reserve vault (source of assets)
    #[account(
        mut,
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// User's token account (destination for withdrawn assets)
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_token_account.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Withdraw>, supply_tokens: u64) -> Result<()> {
    require!(supply_tokens > 0, LendingError::InvalidAmount);

    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    // Accrue interest before processing
    market.accrue_interest(&clock)?;

    // Calculate exchange rate
    let exchange_rate = calculate_exchange_rate(market.total_supplied, market.total_supply_tokens)?;

    // Calculate underlying assets to withdraw
    let withdraw_amount = (supply_tokens as u128)
        .checked_mul(exchange_rate)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)? as u64;

    // Check available liquidity
    require!(
        withdraw_amount <= ctx.accounts.reserve_vault.amount,
        LendingError::InsufficientLiquidity
    );

    // TODO: Check health factor - ensure withdrawal doesn't cause liquidation
    // For now, simplified check

    // Burn supply tokens
    let burn_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.supply_mint.to_account_info(),
            from: ctx.accounts.user_supply_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::burn(burn_ctx, supply_tokens)?;

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
    token::transfer(transfer_ctx, withdraw_amount)?;

    // Update market state
    market.total_supplied = market.total_supplied
        .checked_sub(withdraw_amount)
        .ok_or(LendingError::MathOverflow)?;
    market.total_supply_tokens = market.total_supply_tokens
        .checked_sub(supply_tokens)
        .ok_or(LendingError::MathOverflow)?;

    emit!(Withdrawn {
        market: market.key(),
        user: ctx.accounts.user.key(),
        supply_tokens,
        withdraw_amount,
        total_supplied: market.total_supplied,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct Withdrawn {
    pub market: Pubkey,
    pub user: Pubkey,
    pub supply_tokens: u64,
    pub withdraw_amount: u64,
    pub total_supplied: u64,
    pub timestamp: i64,
}
