use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::LendingError;

/// Calculate utilization rate (borrowed / total supplied)
/// Returns basis points (0-10000)
pub fn calculate_utilization_rate(
    total_borrowed: u64,
    total_supplied: u64,
) -> Result<u16> {
    if total_supplied == 0 {
        return Ok(0);
    }

    let utilization = (total_borrowed as u128)
        .checked_mul(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(total_supplied as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(utilization as u16)
}

/// Calculate borrow interest rate based on utilization
/// Uses a piecewise linear model:
/// - Below optimal: base_rate + slope1 * (util / optimal)
/// - Above optimal: base_rate + slope1 + slope2 * ((util - optimal) / (1 - optimal))
pub fn calculate_borrow_rate(utilization_bps: u16) -> Result<u64> {
    let optimal_util = OPTIMAL_UTILIZATION_BPS as u64;
    let util = utilization_bps as u64;

    let rate = if util <= optimal_util {
        // Below optimal: linear increase
        let rate_per_second = BASE_RATE_PER_SECOND
            .checked_add(
                SLOPE_1_PER_SECOND
                    .checked_mul(util)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(optimal_util)
                    .ok_or(LendingError::MathOverflow)?,
            )
            .ok_or(LendingError::MathOverflow)?;
        rate_per_second
    } else {
        // Above optimal: steeper increase
        let excess_util = util
            .checked_sub(optimal_util)
            .ok_or(LendingError::MathOverflow)?;
        let excess_util_ratio = (excess_util as u128)
            .checked_mul(INTEREST_SCALE)
            .ok_or(LendingError::MathOverflow)?
            .checked_div((BPS_SCALE as u64 - optimal_util) as u128)
            .ok_or(LendingError::MathOverflow)?;

        let excess_rate = (SLOPE_2_PER_SECOND as u128)
            .checked_mul(excess_util_ratio)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(LendingError::MathOverflow)?;

        BASE_RATE_PER_SECOND
            .checked_add(SLOPE_1_PER_SECOND)
            .ok_or(LendingError::MathOverflow)?
            .checked_add(excess_rate as u64)
            .ok_or(LendingError::MathOverflow)?
    };

    Ok(rate)
}

/// Calculate supply interest rate from borrow rate
/// Supply rate = borrow_rate * utilization * (1 - protocol_fee)
pub fn calculate_supply_rate(
    borrow_rate: u64,
    utilization_bps: u16,
) -> Result<u64> {
    let utilization = utilization_bps as u128;
    let protocol_fee = PROTOCOL_FEE_BPS as u128;

    let supply_rate = (borrow_rate as u128)
        .checked_mul(utilization)
        .ok_or(LendingError::MathOverflow)?
        .checked_mul(BPS_SCALE as u128 - protocol_fee)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(supply_rate as u64)
}

/// Calculate accrued interest using compound interest formula
/// new_amount = old_amount * (1 + rate_per_second) ^ seconds_elapsed
/// Simplified for on-chain: new_amount = old_amount * (1 + rate * seconds / scale)
pub fn calculate_accrued_interest(
    principal: u64,
    rate_per_second: u64,
    seconds_elapsed: u64,
) -> Result<u64> {
    if seconds_elapsed == 0 {
        return Ok(principal);
    }

    // Simplified compound interest: principal * (1 + rate * time)
    // Using fixed-point math with INTEREST_SCALE
    let interest_factor = (rate_per_second as u128)
        .checked_mul(seconds_elapsed as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)?;

    let new_amount = (principal as u128)
        .checked_mul(INTEREST_SCALE + interest_factor)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)?;

    Ok(new_amount as u64)
}

/// Calculate health factor
/// Health Factor = (collateral_value * liquidation_threshold) / total_borrowed_value
/// Returns basis points (10000 = 1.0, healthy)
pub fn calculate_health_factor(
    collateral_value: u64,
    liquidation_threshold_bps: u16,
    borrowed_value: u64,
) -> Result<u16> {
    if borrowed_value == 0 {
        // No borrows = infinite health factor, return max
        return Ok(u16::MAX);
    }

    let adjusted_collateral = (collateral_value as u128)
        .checked_mul(liquidation_threshold_bps as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?;

    let health_factor = (adjusted_collateral as u128)
        .checked_mul(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(borrowed_value as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(health_factor as u16)
}

/// Calculate maximum borrow amount given collateral
/// max_borrow = (collateral_value * LTV) / token_price
pub fn calculate_max_borrow(
    collateral_value: u64,
    ltv_bps: u16,
    token_price: u64,
) -> Result<u64> {
    let max_borrow_value = (collateral_value as u128)
        .checked_mul(ltv_bps as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?;

    let max_borrow_amount = max_borrow_value
        .checked_div(token_price as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(max_borrow_amount as u64)
}

/// Calculate liquidation bonus amount
/// bonus = amount * liquidation_bonus_bps / BPS_SCALE
pub fn calculate_liquidation_bonus(amount: u64) -> Result<u64> {
    let bonus = (amount as u128)
        .checked_mul(LIQUIDATION_BONUS_BPS as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BPS_SCALE as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(bonus as u64)
}

/// Calculate exchange rate for yield-bearing tokens
/// exchange_rate = total_supplied / total_supply_tokens
pub fn calculate_exchange_rate(
    total_supplied: u64,
    total_supply_tokens: u64,
) -> Result<u128> {
    if total_supply_tokens == 0 {
        // Initial exchange rate: 1:1
        return Ok(INTEREST_SCALE);
    }

    let rate = (total_supplied as u128)
        .checked_mul(INTEREST_SCALE)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(total_supply_tokens as u128)
        .ok_or(LendingError::MathOverflow)?;

    Ok(rate)
}
