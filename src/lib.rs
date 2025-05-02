pub mod curve;
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export items for processor
use solana_program::declare_id;

// Define the program ID. Replace with your actual deployed program ID.
declare_id!("TokenLaunch11111111111111111111111111111111"); 