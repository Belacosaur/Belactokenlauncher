use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Enum representing the state of the bonding curve
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum BondingCurveState {
    Active,
    LockedForMigration,
    Migrated,
    Failed, // Optional: For handling migration failures
}

/// Configuration account data for the bonding curve
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BondingCurveConfig {
    /// The authority allowed to manage sensitive operations (like triggering migration completion)
    pub authority: Pubkey,
    /// Current state of the bonding curve
    pub state: BondingCurveState,
    /// The mint address of the token being traded
    pub token_mint: Pubkey,
    /// The PDA account holding SOL reserves
    pub sol_vault: Pubkey,
    /// The PDA account holding the contract's token supply
    pub token_vault: Pubkey,
    /// Virtual SOL reserves used for price calculation
    pub virtual_sol_reserves: u64,
    /// Virtual token reserves used for price calculation
    pub virtual_token_reserves: u64,
    /// Real SOL reserves held in the sol_vault (primarily for tracking/migration threshold)
    pub real_sol_reserves: u64, // Can also be derived from sol_vault.lamports()
    /// Tracks the total number of tokens sold via the curve
    pub total_supply_sold: u64,
    /// The wallet address of the token creator
    pub creator: Pubkey,
    /// Fee basis points for the creation fee (e.g., 100 = 1%)
    pub creation_fee_basis_points: u16,
    /// Fee basis points for buy/sell trades (e.g., 100 = 1%)
    pub trade_fee_basis_points: u16,
    /// The SOL amount in the real vault that triggers the migration state
    pub market_cap_threshold_sol: u64,
    /// Optional: Stored Raydium pool ID after successful migration
    pub raydium_pool_id: Option<Pubkey>,
    /// Optional: Stored OpenBook market ID after successful migration
    pub openbook_market_id: Option<Pubkey>,
    // Add potential padding if needed for future upgrades
    // _padding: [u8; 64],
}

// Optional: Implement helper to get account size if needed for creation
impl BondingCurveConfig {
    pub const MAX_SIZE: usize = 32 + // authority
                                1 + // state enum discriminant
                                32 + // token_mint
                                32 + // sol_vault
                                32 + // token_vault
                                8 +  // virtual_sol_reserves
                                8 +  // virtual_token_reserves
                                8 +  // real_sol_reserves
                                8 +  // total_supply_sold
                                32 + // creator
                                2 +  // creation_fee_basis_points
                                2 +  // trade_fee_basis_points
                                8 +  // market_cap_threshold_sol
                                (1 + 32) + // Option<Pubkey> raydium_pool_id
                                (1 + 32); // Option<Pubkey> openbook_market_id
                                // + 64; // Optional padding
} 