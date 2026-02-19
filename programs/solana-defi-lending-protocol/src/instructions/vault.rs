use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::LendingError;
use crate::constants::*;

/// Create a vault for automated yield strategies
/// 
/// Vaults allow users to deposit assets that are automatically allocated
/// across multiple markets based on a strategy (conservative, balanced, aggressive).
#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = Vault::BASE_SIZE + 100, // Space for allocations
        seeds = [b"vault", owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateVault>,
    strategy: u8,
    rebalance_threshold_bps: u16,
) -> Result<()> {
    require!(
        strategy <= VaultStrategy::Aggressive as u8,
        LendingError::InvalidVaultAllocation
    );

    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    let bump = ctx.bumps.get("vault").copied().unwrap();

    vault.initialize(
        ctx.accounts.owner.key(),
        strategy,
        rebalance_threshold_bps,
        bump,
        &clock,
    );

    emit!(VaultCreated {
        vault: vault.key(),
        owner: ctx.accounts.owner.key(),
        strategy,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Rebalance vault allocations across markets
/// 
/// This instruction rebalances the vault's asset allocation based on
/// current market conditions and the vault's strategy.
#[derive(Accounts)]
pub struct RebalanceVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", owner.key().as_ref()],
        bump = vault.bump,
        constraint = vault.owner == owner.key() @ LendingError::Unauthorized
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: Multiple markets can be passed for rebalancing
    /// In a full implementation, you'd use remaining_accounts
    pub markets: Vec<AccountInfo<'info>>,
}

pub fn handler(ctx: Context<RebalanceVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;

    // TODO: In a full implementation, this would:
    // 1. Fetch current market conditions (interest rates, utilization, etc.)
    // 2. Calculate optimal allocation based on vault strategy
    // 3. Compare with current allocation
    // 4. If drift exceeds rebalance_threshold_bps, execute rebalance
    // 5. Transfer assets between markets via supply/withdraw

    // For now, just update last_rebalance timestamp
    vault.last_rebalance = clock.unix_timestamp;

    emit!(VaultRebalanced {
        vault: vault.key(),
        owner: ctx.accounts.owner.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

pub fn rebalance_handler(ctx: Context<RebalanceVault>) -> Result<()> {
    handler(ctx)
}

#[event]
pub struct VaultCreated {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub strategy: u8,
    pub timestamp: i64,
}

#[event]
pub struct VaultRebalanced {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}
