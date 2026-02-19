use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;
use crate::math::*;

/// Liquidate an unhealthy borrow position
/// 
/// When a user's health factor drops below the threshold, liquidators can
/// repay their debt at a discount (liquidation bonus) and seize collateral.
#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// Market for the borrowed asset
    #[account(
        mut,
        seeds = [b"market", borrow_market.asset_mint.as_ref()],
        bump = borrow_market.bump
    )]
    pub borrow_market: Account<'info, Market>,

    /// Market for the collateral asset
    #[account(
        mut,
        seeds = [b"market", collateral_market.asset_mint.as_ref()],
        bump = collateral_market.bump
    )]
    pub collateral_market: Account<'info, Market>,

    /// Liquidator's token account (source of repayment)
    #[account(
        mut,
        constraint = liquidator_token_account.owner == liquidator.key() @ LendingError::Unauthorized,
        constraint = liquidator_token_account.mint == borrow_market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub liquidator_token_account: Account<'info, TokenAccount>,

    /// Borrow market reserve vault (destination for repayment)
    #[account(
        mut,
        constraint = borrow_reserve_vault.mint == borrow_market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub borrow_reserve_vault: Account<'info, TokenAccount>,

    /// Collateral market reserve vault (source of seized collateral)
    #[account(
        mut,
        constraint = collateral_reserve_vault.mint == collateral_market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub collateral_reserve_vault: Account<'info, TokenAccount>,

    /// Liquidator's collateral token account (destination for seized collateral)
    #[account(
        mut,
        constraint = liquidator_collateral_account.owner == liquidator.key() @ LendingError::Unauthorized,
        constraint = liquidator_collateral_account.mint == collateral_market.asset_mint @ LendingError::InvalidMarketConfig
    )]
    pub liquidator_collateral_account: Account<'info, TokenAccount>,

    /// CHECK: Oracle for borrowed asset price
    pub borrow_oracle: UncheckedAccount<'info>,

    /// CHECK: Oracle for collateral asset price
    pub collateral_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<Liquidate>,
    repay_amount: u64,
    min_collateral_amount: u64,
) -> Result<()> {
    require!(repay_amount > 0, LendingError::InvalidAmount);

    let borrow_market = &mut ctx.accounts.borrow_market;
    let collateral_market = &mut ctx.accounts.collateral_market;
    let clock = Clock::get()?;

    // Accrue interest on both markets
    borrow_market.accrue_interest(&clock)?;
    collateral_market.accrue_interest(&clock)?;

    // TODO: In a full implementation, you would:
    // 1. Fetch user's borrow position
    // 2. Calculate their health factor using oracle prices
    // 3. Verify health factor < MIN_HEALTH_FACTOR_BPS
    // 4. Calculate collateral to seize based on prices and liquidation bonus
    // 5. Verify min_collateral_amount is met

    // For now, simplified liquidation flow:
    // Transfer repayment from liquidator to reserve
    let borrow_seeds = &[
        b"market",
        borrow_market.asset_mint.as_ref(),
        &[borrow_market.bump],
    ];
    let borrow_signer = &[&borrow_seeds[..]];

    let repay_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.liquidator_token_account.to_account_info(),
            to: ctx.accounts.borrow_reserve_vault.to_account_info(),
            authority: ctx.accounts.liquidator.to_account_info(),
        },
    );
    token::transfer(repay_ctx, repay_amount)?;

    // Calculate collateral to seize (with liquidation bonus)
    // Simplified: assume 1:1 price ratio for now
    // In production, use oracle prices
    let collateral_amount = repay_amount
        .checked_add(calculate_liquidation_bonus(repay_amount)?)
        .ok_or(LendingError::MathOverflow)?;

    require!(
        collateral_amount >= min_collateral_amount,
        LendingError::SlippageExceeded
    );

    // Transfer collateral from reserve to liquidator
    let collateral_seeds = &[
        b"market",
        collateral_market.asset_mint.as_ref(),
        &[collateral_market.bump],
    ];
    let collateral_signer = &[&collateral_seeds[..]];

    let seize_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.collateral_reserve_vault.to_account_info(),
            to: ctx.accounts.liquidator_collateral_account.to_account_info(),
            authority: collateral_market.to_account_info(),
        },
        collateral_signer,
    );
    token::transfer(seize_ctx, collateral_amount)?;

    // Update market states
    borrow_market.total_borrowed = borrow_market.total_borrowed
        .checked_sub(repay_amount)
        .ok_or(LendingError::MathOverflow)?;
    collateral_market.total_supplied = collateral_market.total_supplied
        .checked_sub(collateral_amount)
        .ok_or(LendingError::MathOverflow)?;

    emit!(Liquidated {
        borrow_market: borrow_market.key(),
        collateral_market: collateral_market.key(),
        liquidator: ctx.accounts.liquidator.key(),
        repay_amount,
        collateral_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct Liquidated {
    pub borrow_market: Pubkey,
    pub collateral_market: Pubkey,
    pub liquidator: Pubkey,
    pub repay_amount: u64,
    pub collateral_amount: u64,
    pub timestamp: i64,
}
