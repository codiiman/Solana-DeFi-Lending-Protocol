use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Invalid market configuration")]
    InvalidMarketConfig,

    #[msg("Market not found")]
    MarketNotFound,

    #[msg("Insufficient collateral")]
    InsufficientCollateral,

    #[msg("Borrow limit exceeded")]
    BorrowLimitExceeded,

    #[msg("Insufficient liquidity in market")]
    InsufficientLiquidity,

    #[msg("Health factor too low")]
    HealthFactorTooLow,

    #[msg("Liquidation not needed - health factor is safe")]
    LiquidationNotNeeded,

    #[msg("Invalid liquidation amount")]
    InvalidLiquidationAmount,

    #[msg("Oracle price is stale")]
    StaleOraclePrice,

    #[msg("Invalid oracle account")]
    InvalidOracle,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid interest rate calculation")]
    InvalidInterestRate,

    #[msg("Market is paused")]
    MarketPaused,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Reserve not found")]
    ReserveNotFound,

    #[msg("Borrow position not found")]
    BorrowPositionNotFound,

    #[msg("Cannot withdraw - would cause health factor to drop below threshold")]
    WithdrawWouldCauseLiquidation,

    #[msg("Cannot borrow - would cause health factor to drop below threshold")]
    BorrowWouldCauseLiquidation,

    #[msg("Vault strategy not found")]
    VaultStrategyNotFound,

    #[msg("Invalid vault allocation")]
    InvalidVaultAllocation,

    #[msg("Vault rebalance not needed")]
    VaultRebalanceNotNeeded,

    #[msg("Interest accrual failed")]
    InterestAccrualFailed,

    #[msg("Invalid utilization rate")]
    InvalidUtilizationRate,

    #[msg("Market already initialized")]
    MarketAlreadyInitialized,

    #[msg("Invalid LTV ratio")]
    InvalidLtvRatio,

    #[msg("Invalid liquidation threshold")]
    InvalidLiquidationThreshold,

    #[msg("Liquidation threshold must be greater than LTV")]
    LiquidationThresholdTooLow,
}
