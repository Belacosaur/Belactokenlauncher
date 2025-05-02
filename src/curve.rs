use solana_program::program_error::ProgramError;
use crate::error::TokenLaunchError;

/// Calculate the amount of tokens to receive when buying with SOL
/// Uses a constant product formula on virtual reserves: (x + dx)(y - dy) = k
/// where x = virtual_sol, y = virtual_token, dx = sol_amount_in, dy = token_amount_out
pub fn calculate_buy_amount(
    sol_amount_in: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    trade_fee_basis_points: u16,
) -> Result<u64, ProgramError> {
    if virtual_sol_reserves == 0 || virtual_token_reserves == 0 {
        // Avoid division by zero, or handle initial liquidity case
        return Err(TokenLaunchError::InvalidCurveState.into()); 
    }

    // k = x * y
    let k = (virtual_sol_reserves as u128)
        .checked_mul(virtual_token_reserves as u128)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Calculate fee on input amount
    let fee_amount = calculate_fee(sol_amount_in, trade_fee_basis_points)?;
    let sol_amount_in_after_fee = sol_amount_in
        .checked_sub(fee_amount)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // New virtual SOL amount: x + dx
    let new_virtual_sol = virtual_sol_reserves
        .checked_add(sol_amount_in_after_fee)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Calculate new virtual token amount: y' = k / (x + dx)
    let new_virtual_token = k
        .checked_div(new_virtual_sol as u128)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Amount of tokens out: dy = y - y'
    let token_amount_out = (virtual_token_reserves as u128)
        .checked_sub(new_virtual_token)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    Ok(token_amount_out as u64)
}

/// Calculate the amount of SOL to receive when selling tokens AND the fee taken
/// Uses a constant product formula: (x - dx)(y + dy) = k
/// where x = virtual_sol, y = virtual_token, dy = token_amount_in, dx = sol_amount_out_gross
pub fn calculate_sell_amount(
    token_amount_in: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    trade_fee_basis_points: u16,
) -> Result<(u64, u64, u64), ProgramError> {
     if virtual_sol_reserves == 0 || virtual_token_reserves == 0 {
        return Err(TokenLaunchError::InvalidCurveState.into());
    }

    // k = x * y
    let k = (virtual_sol_reserves as u128)
        .checked_mul(virtual_token_reserves as u128)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // New virtual token amount: y + dy
    let new_virtual_token = virtual_token_reserves
        .checked_add(token_amount_in)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Calculate new virtual SOL amount: x' = k / (y + dy)
    let new_virtual_sol = k
        .checked_div(new_virtual_token as u128)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Amount of SOL out (before fee): dx = x - x'
    let sol_amount_out_gross = (virtual_sol_reserves as u128)
        .checked_sub(new_virtual_sol)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    // Calculate fee on output amount
    let fee_amount = calculate_fee(sol_amount_out_gross as u64, trade_fee_basis_points)?;
    
    // Net SOL amount out
    let sol_amount_out_net = (sol_amount_out_gross as u64)
        .checked_sub(fee_amount)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;

    Ok((sol_amount_out_net, sol_amount_out_gross as u64, fee_amount))
}

/// Helper to calculate fee amount based on basis points
pub fn calculate_fee(amount: u64, fee_basis_points: u16) -> Result<u64, ProgramError> {
    if fee_basis_points == 0 {
        return Ok(0);
    }
    // Basis points: 100 = 1%, 10000 = 100%
    let fee = (amount as u128)
        .checked_mul(fee_basis_points as u128)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(TokenLaunchError::ArithmeticOverflow)?;
    Ok(fee as u64)
}

// TODO: Add unit tests for these functions 