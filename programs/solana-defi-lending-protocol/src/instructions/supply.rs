use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo, Transfer};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;
use crate::math::*;

/// Supply assets to a lending market
/// 
/// Users supply assets and receive yield-bearing tokens (supply tokens)
/// that represent their share of the market plus accrued interest.
#[derive(Accounts)]
pub struct Supply<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.asset_mint.as_ref()],
        bump = market.bump,
        constraint = !market.paused @ LendingError::MarketPaused
    )]
    pub market: Account<'info, Market>,

    /// User's token account (source of supply)
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_token_account.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Reserve vault (destination for supplied assets)
    #[account(
        mut,
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// Supply mint (yield-bearing tokens)
    #[account(
        mut,
        constraint = supply_mint.key() == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub supply_mint: Account<'info, Mint>,

    /// User's supply token account (receives yield-bearing tokens)
    #[account(
        mut,
        constraint = user_supply_account.mint == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_supply_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Supply>, amount: u64) -> Result<()> {
    require!(amount >= MIN_SUPPLY_AMOUNT, LendingError::InvalidAmount);

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

    // Calculate exchange rate
    let exchange_rate = if market.total_supply_tokens == 0 {
        INTEREST_SCALE // 1:1 initial rate
    } else {
        calculate_exchange_rate(market.total_supplied, market.total_supply_tokens)?
    };

    // Calculate supply tokens to mint
    let supply_tokens = (amount as u128)
        .checked_mul(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(exchange_rate)
        .ok_or(LendingError::MathOverflow)? as u64;

    // Mint supply tokens to user
    let seeds = &[
        b"market",
        market.asset_mint.as_ref(),
        &[market.bump],
    ];
    let signer = &[&seeds[..]];

    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.supply_mint.to_account_info(),
            to: ctx.accounts.user_supply_account.to_account_info(),
            authority: market.to_account_info(),
        },
        signer,
    );
    token::mint_to(mint_ctx, supply_tokens)?;

    // Update market state
    market.total_supplied = market.total_supplied
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;
    market.total_supply_tokens = market.total_supply_tokens
        .checked_add(supply_tokens)
        .ok_or(LendingError::MathOverflow)?;

    emit!(Supplied {
        market: market.key(),
        user: ctx.accounts.user.key(),
        amount,
        supply_tokens,
        total_supplied: market.total_supplied,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct Supplied {
    pub market: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub supply_tokens: u64,
    pub total_supplied: u64,
    pub timestamp: i64,
}
