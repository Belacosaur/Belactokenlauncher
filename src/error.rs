use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum TokenLaunchError {
    /// Error deserializing instruction data
    #[error("Instruction Unpack Error")]
    InstructionUnpackError,

    /// Invalid authority provided
    #[error("Invalid Authority")]
    InvalidAuthority,

    /// Arithmetic overflow occurred
    #[error("Arithmetic Overflow")]
    ArithmeticOverflow,

    /// Curve is not in the expected state for the operation
    #[error("Invalid Curve State")]
    InvalidCurveState,

    /// Insufficient SOL in vault for payout
    #[error("Insufficient SOL Balance")]
    InsufficientSolBalance,

    /// Calculation resulted in zero output amount
    #[error("Calculation Error")]
    CalculationError,

    /// Insufficient SOL funds in vault for payout
    #[error("Insufficient Funds")]
    InsufficientFunds,

    // Add more specific errors as needed...
}

// Implement conversion from custom error to Solana ProgramError
impl From<TokenLaunchError> for ProgramError {
    fn from(e: TokenLaunchError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
