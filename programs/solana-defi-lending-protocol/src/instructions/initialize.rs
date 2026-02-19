use anchor_lang::prelude::*;
use crate::state::*;

/// Initialize the global protocol configuration
/// 
/// This must be called once by the protocol authority to set up
/// the global configuration and treasury PDA.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = GlobalConfig::SIZE,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK: Treasury PDA validated by seeds
    #[account(
        mut,
        seeds = [b"treasury", global_config.key().as_ref()],
        bump
    )]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let treasury_bump = ctx.bumps.get("treasury").copied().unwrap();

    global_config.initialize(
        ctx.accounts.authority.key(),
        ctx.accounts.treasury.key(),
        treasury_bump,
    );

    emit!(ProtocolInitialized {
        authority: ctx.accounts.authority.key(),
        treasury: ctx.accounts.treasury.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub timestamp: i64,
}
