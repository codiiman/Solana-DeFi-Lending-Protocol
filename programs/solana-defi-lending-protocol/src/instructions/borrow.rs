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
        constraint = reserve_vault.mint == market.asset_mint @ LendingError::InvalidMarketConfig,
        constraint = reserve_vault.key() == market.reserve_vault @ LendingError::InvalidReserveVault
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    /// User's token account (destination for borrowed assets)
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_token_account.mint == market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User's supply token account (collateral)
    #[account(
        constraint = user_supply_account.owner == user.key() @ LendingError::Unauthorized,
        constraint = user_supply_account.mint == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub user_supply_account: Account<'info, TokenAccount>,

    /// Supply mint (for exchange rate calculation)
    #[account(
        constraint = supply_mint.key() == market.supply_mint @ LendingError::InvalidMarketConfig
    )]
    pub supply_mint: Account<'info, Mint>,

    /// User's borrow position PDA (initialized if first borrow)
    #[account(
        init_if_needed,
        payer = user,
        space = BorrowPosition::SIZE,
        seeds = [b"borrow_position", market.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub borrow_position: Account<'info, BorrowPosition>,

    /// CHECK: Oracle account for price feed — validated against market.oracle
    #[account(
        constraint = oracle.key() == market.oracle @ LendingError::InvalidOracle
    )]
    pub oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
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

    // Calculate user's collateral value in terms of the underlying asset
    let exchange_rate = if market.total_supply_tokens == 0 {
        INTEREST_SCALE
    } else {
        calculate_exchange_rate(market.total_supplied, market.total_supply_tokens)?
    };

    let user_collateral = (ctx.accounts.user_supply_account.amount as u128)
        .checked_mul(exchange_rate)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)? as u64;

    // Calculate current debt (if any existing position)
    let current_debt = if ctx.accounts.borrow_position.borrowed_amount > 0 {
        ctx.accounts.borrow_position.calculate_debt(market)?
    } else {
        0
    };

    // Check collateral: new borrow + existing debt must not exceed LTV * collateral
    let new_total_debt = current_debt
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;

    let max_borrow = (user_collateral as u128)
        .checked_mul(market.ltv_bps as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10000)
        .ok_or(LendingError::MathOverflow)? as u64;

    require!(
        new_total_debt <= max_borrow,
        LendingError::InsufficientCollateral
    );

    // Calculate new total borrowed for market
    let new_market_total_borrowed = market.total_borrowed
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;

    // Check utilization doesn't exceed 100%
    let utilization_bps = calculate_utilization_rate(new_market_total_borrowed, market.total_supplied)?;
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
    market.total_borrowed = new_market_total_borrowed;

    // Initialize or update borrow position
    let position = &mut ctx.accounts.borrow_position;
    if position.borrowed_amount == 0 {
        position.initialize(
            ctx.accounts.user.key(),
            market.key(),
            amount,
            market.cumulative_borrow_rate,
            &clock,
        );
    } else {
        // Update existing position with new cumulative rate and combined debt
        position.borrowed_amount = new_total_debt;
        position.cumulative_borrow_rate_snapshot = market.cumulative_borrow_rate;
        position.last_updated = clock.unix_timestamp;
    }

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
