use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Instruction arguments for `CreateTokenAndBondingCurve`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct CreateArgs {
    pub name: String,
    pub symbol: String,
    pub metadata_uri: String,
    pub market_cap_threshold_sol: u64,
    pub initial_virtual_sol: u64,
    pub initial_virtual_token: u64,
    // Consider adding total_initial_supply if not fixed
}

/// Instruction arguments for `BuyToken`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct BuyArgs {
    pub sol_amount_in: u64,
    // pub min_token_amount_out: u64, // Optional slippage protection
}

/// Instruction arguments for `SellToken`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct SellArgs {
    pub token_amount_in: u64,
    // pub min_sol_amount_out: u64, // Optional slippage protection
}

/// Instruction arguments for `TransferLiquidityAndCompleteMigration`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct TransferArgs {
    pub raydium_pool_id: Option<Pubkey>,
    pub openbook_market_id: Option<Pubkey>,
}

/// Instruction arguments for `UpdateTradeFee`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateTradeFeeArgs {
    pub new_trade_fee_basis_points: u16,
}

/// Enum representing the different instructions the program can handle
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum TokenLaunchInstruction {
    /// Creates a new SPL token, metadata, bonding curve config, and vaults.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` Payer and creator wallet.
    /// 1. `[writable]` BondingCurveConfig account (must be uninitialized PDA).
    /// 2. `[writable]` Token Mint account (must be uninitialized).
    /// 3. `[writable]` SOL Vault PDA account (must be uninitialized PDA).
    /// 4. `[writable]` Token Vault PDA account (must be uninitialized PDA).
    /// 5. `[writable]` Metaplex Metadata account (must be uninitialized PDA).
    /// 6. `[]` System Program.
    /// 7. `[]` Token Program.
    /// 8. `[]` Metaplex Token Metadata Program.
    /// 9. `[]` Rent Sysvar.
    CreateTokenAndBondingCurve(CreateArgs),

    /// Buys tokens from the bonding curve using SOL.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` User wallet buying tokens.
    /// 1. `[writable]` User's Associated Token Account (ATA) for the token mint.
    /// 2. `[writable]` BondingCurveConfig account (PDA).
    /// 3. `[writable]` SOL Vault PDA account.
    /// 4. `[writable]` Token Vault PDA account.
    /// 5. `[]` Token Mint account.
    /// 6. `[]` System Program.
    /// 7. `[]` Token Program.
    BuyToken(BuyArgs),

    /// Sells tokens back to the bonding curve for SOL.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` User wallet selling tokens.
    /// 1. `[writable]` User's Associated Token Account (ATA) for the token mint.
    /// 2. `[writable]` BondingCurveConfig account (PDA).
    /// 3. `[writable]` SOL Vault PDA account.
    /// 4. `[writable]` Token Vault PDA account.
    /// 5. `[]` Token Mint account.
    /// 6. `[]` Token Program.
    SellToken(SellArgs),

    /// Transfers liquidity from vaults to designated accounts (called by backend authority).
    ///
    /// Accounts expected:
    /// 0. `[signer]` Authority wallet (must match config.authority).
    /// 1. `[writable]` BondingCurveConfig account (PDA).
    /// 2. `[writable]` SOL Vault PDA account.
    /// 3. `[writable]` Token Vault PDA account.
    /// 4. `[writable]` Destination SOL account (e.g., Raydium authority or temp account).
    /// 5. `[writable]` Destination Token account (e.g., Raydium authority or temp account).
    /// 6. `[]` System Program.
    /// 7. `[]` Token Program.
    TransferLiquidityAndCompleteMigration(TransferArgs),

    /// Updates the trade fee basis points. Called by the config authority.
    /// 
    /// Accounts expected:
    /// 0. `[signer]` Authority account (must match config.authority).
    /// 1. `[writable]` Config account PDA.
    UpdateTradeFee(UpdateTradeFeeArgs),
}

impl TokenLaunchInstruction {
    /// Unpacks instruction data byte slice into the instruction enum
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Split the input buffer into the instruction variant index and the rest
        let (&variant_index, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        
        Ok(match variant_index {
            0 => Self::CreateTokenAndBondingCurve(CreateArgs::try_from_slice(rest)?),
            1 => Self::BuyToken(BuyArgs::try_from_slice(rest)?),
            2 => Self::SellToken(SellArgs::try_from_slice(rest)?),
            3 => Self::TransferLiquidityAndCompleteMigration(TransferArgs::try_from_slice(rest)?),
            4 => Self::UpdateTradeFee(UpdateTradeFeeArgs::try_from_slice(rest)?),
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
    
    // Helper to pack instruction data (optional, useful for client-side)
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Self::CreateTokenAndBondingCurve(args) => {
                buf.push(0);
                buf.extend_from_slice(&args.try_to_vec().unwrap());
            }
            Self::BuyToken(args) => {
                buf.push(1);
                buf.extend_from_slice(&args.try_to_vec().unwrap());
            }
            Self::SellToken(args) => {
                buf.push(2);
                buf.extend_from_slice(&args.try_to_vec().unwrap());
            }
            Self::TransferLiquidityAndCompleteMigration(args) => {
                buf.push(3);
                buf.extend_from_slice(&args.try_to_vec().unwrap());
            }
            Self::UpdateTradeFee(args) => {
                buf.push(4);
                buf.extend_from_slice(&args.try_to_vec().unwrap());
            }
        }
        buf
    }
} 