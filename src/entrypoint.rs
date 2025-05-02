use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
};

use crate::processor::Processor;

// Declare the program entrypoint
entrypoint!(process_instruction);

/// Program entrypoint handler
pub fn process_instruction(
    program_id: &Pubkey,      // Public key of the program
    accounts: &[AccountInfo], // Accounts involved in the transaction
    instruction_data: &[u8], // Instruction data passed by the client
) -> ProgramResult {
    msg!("Entrypoint: Processing instruction...");

    // Delegate processing to the Processor module
    Processor::process(program_id, accounts, instruction_data)
} 