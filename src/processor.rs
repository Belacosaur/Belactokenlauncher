use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
    program_pack::Pack,
};
use spl_token::instruction as token_instruction;
// Note: Correct import path for metaplex might vary slightly based on crate version features
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use mpl_token_metadata::instruction as metadata_instruction;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::instruction::{CreateArgs, BuyArgs, SellArgs, TransferArgs, UpdateTradeFeeArgs, TokenLaunchInstruction};
use crate::error::TokenLaunchError;
use crate::state::{BondingCurveConfig, BondingCurveState};
// TODO: Uncomment these when implementing Buy/Sell instructions
use crate::curve::{calculate_buy_amount, calculate_sell_amount}; // Removed calculate_fee

// Define the hardcoded fee update authority pubkey
const FEE_UPDATE_AUTHORITY: Pubkey = pubkey!("CtvCore6RGRr7pXnbM34mFbaYxeiELj3JHQ5gefxhi7t");

pub struct Processor;
impl Processor {
    /// Main processing function routing instructions
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // Unpack the instruction data
        let instruction = TokenLaunchInstruction::unpack(instruction_data)?;

        // Route based on the instruction variant
        match instruction {
            TokenLaunchInstruction::CreateTokenAndBondingCurve(args) => {
                msg!("Instruction: CreateTokenAndBondingCurve");
                // Log the unpacked arguments IMMEDIATELY after unpack
                msg!("Unpacked Create Args - Name: {}", args.name);
                msg!("Unpacked Create Args - Symbol: {}", args.symbol);
                msg!("Unpacked Create Args - URI: {}", args.metadata_uri);
                msg!("Unpacked Create Args - Market Cap SOL: {}", args.market_cap_threshold_sol);
                msg!("Unpacked Create Args - Initial Virtual SOL: {}", args.initial_virtual_sol_reserves);
                msg!("Unpacked Create Args - Initial Virtual Token: {}", args.initial_virtual_token_reserves);
                msg!("Unpacked Create Args - Trade Fee BPS: {}", args.trade_fee_basis_points);
                msg!("Unpacked Create Args - Creator Fee BPS: {}", args.creator_fee_basis_points);

                // === ADDING ARGUMENT VALIDATION ===
                // If these are zero, it means deserialization likely failed, despite matching schemas.
                // We expect non-zero values based on frontend logic.
                if args.initial_virtual_sol_reserves == 0 {
                    msg!("Validation Error: Unpacked initial_virtual_sol_reserves is zero!");
                    return Err(ProgramError::InvalidArgument);
                }
                if args.initial_virtual_token_reserves == 0 {
                    msg!("Validation Error: Unpacked initial_virtual_token_reserves is zero!");
                    return Err(ProgramError::InvalidArgument);
                }
                 // Allow zero creator fee, but trade fee should generally be non-zero if set
                 // Let's assume trade_fee > 0 was sent if it's non-zero in the form
                 // (Add more specific checks if needed based on form validation)
                 // For now, focus on reserves which MUST be non-zero.

                Self::process_create_token_and_bonding_curve(program_id, accounts, args)
            }
            TokenLaunchInstruction::BuyToken(args) => {
                msg!("Instruction: BuyToken");
                Self::process_buy_token(program_id, accounts, args)
            }
            TokenLaunchInstruction::SellToken(args) => {
                msg!("Instruction: SellToken");
                Self::process_sell_token(program_id, accounts, args)
            }
            TokenLaunchInstruction::TransferLiquidityAndCompleteMigration(args) => {
                msg!("Instruction: TransferLiquidityAndCompleteMigration");
                Self::process_transfer_liquidity(program_id, accounts, args)
            }
            TokenLaunchInstruction::UpdateTradeFee(args) => {
                msg!("Instruction: UpdateTradeFee");
                Self::process_update_trade_fee(program_id, accounts, args)
            }
        }
    }

    /// Helper function to perform CPI calls for account creation and initialization
    fn create_and_initialize_accounts<'a>(
        program_id: &Pubkey,
        payer_creator_account: &AccountInfo<'a>,
        config_account: &AccountInfo<'a>,
        token_mint_account: &AccountInfo<'a>,
        sol_vault_account: &AccountInfo<'a>,
        token_vault_account: &AccountInfo<'a>,
        metadata_account: &AccountInfo<'a>,
        system_program_account: &AccountInfo<'a>,
        token_program_account: &AccountInfo<'a>,
        metadata_program_account: &AccountInfo<'a>,
        rent_sysvar_account: &AccountInfo<'a>,
        config_pda: &Pubkey,
        config_signer_seeds: &[&[u8]],
        sol_vault_signer_seeds: &[&[u8]],
        token_vault_signer_seeds: &[&[u8]],
        rent: &Rent,
        args: &CreateArgs, // Pass args by reference
    ) -> ProgramResult {
        msg!("Performing CPI calls (Helper function)...");

        // Calculate required lamports for rent exemption
        let mint_rent_lamports = rent.minimum_balance(spl_token::state::Mint::LEN);
        let vault_rent_lamports = rent.minimum_balance(spl_token::state::Account::LEN);
        // Define the maximum possible Borsh serialized size for the config
        const MAX_CONFIG_SIZE: usize = 32 + 1 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 32 + 2 + 2 + 8 + (1 + 32) + (1 + 32); // 261 bytes
        msg!("Using max config account size: {}", MAX_CONFIG_SIZE);
        let config_rent_lamports = rent.minimum_balance(MAX_CONFIG_SIZE);

        // 0. CPI: Create Config Account
        msg!("Creating Config Account PDA...");
        invoke_signed(
            &system_instruction::create_account(
                payer_creator_account.key,
                config_account.key,
                config_rent_lamports,
                MAX_CONFIG_SIZE as u64, // Use MAX_CONFIG_SIZE
                program_id,
            ),
            &[
                payer_creator_account.clone(),
                config_account.clone(),
                system_program_account.clone(),
            ],
            &[config_signer_seeds],
        )?;

        // 1. CPI: Create SOL Vault Account
        msg!("Creating SOL Vault PDA...");
         invoke_signed(
            &system_instruction::create_account(
                payer_creator_account.key,
                sol_vault_account.key,
                vault_rent_lamports, 
                0, 
                program_id,
            ),
            &[
                payer_creator_account.clone(),
                sol_vault_account.clone(),
                system_program_account.clone(),
            ],
            &[sol_vault_signer_seeds],
        )?;

        // 2. CPI: Create Token Mint Account
        msg!("Creating Token Mint Account...");
        invoke(
            &system_instruction::create_account(
                payer_creator_account.key,
                token_mint_account.key,
                mint_rent_lamports,
                spl_token::state::Mint::LEN as u64,
                token_program_account.key,
            ),
            &[
                payer_creator_account.clone(),
                token_mint_account.clone(),
                system_program_account.clone(),
            ],
        )?;

        // 3. CPI: Initialize Token Mint
        msg!("Initializing Token Mint...");
        let token_decimals = 6;
        invoke_signed(
            &token_instruction::initialize_mint(
                token_program_account.key,
                token_mint_account.key,
                config_pda,
                None,
                token_decimals,
            )?,
            &[
                token_mint_account.clone(),
                rent_sysvar_account.clone(),
                config_account.clone(),
            ],
            &[config_signer_seeds],
        )?;

        // 4. CPI: Create Token Vault Account
        msg!("Creating Token Vault PDA...");
        invoke_signed(
            &system_instruction::create_account(
                payer_creator_account.key,
                token_vault_account.key,
                vault_rent_lamports,
                spl_token::state::Account::LEN as u64,
                token_program_account.key,
            ),
            &[
                payer_creator_account.clone(),
                token_vault_account.clone(),
                system_program_account.clone(),
            ],
            &[token_vault_signer_seeds],
        )?;

        // 5. CPI: Initialize Token Vault
        msg!("Initializing Token Vault...");
        invoke(
            &token_instruction::initialize_account(
                token_program_account.key,
                token_vault_account.key,
                token_mint_account.key,
                config_pda,
            )?,
            &[
                token_vault_account.clone(),
                token_mint_account.clone(),
                config_account.clone(),
                rent_sysvar_account.clone(),
                token_program_account.clone(),
            ],
        )?;

        // 6. CPI: Mint Initial Tokens to Token Vault
        msg!("Minting Initial Tokens...");
        let initial_supply = 1_000_000_000_000; // TODO: Needs clarification if this should relate to virtual reserves
        invoke_signed(
            &token_instruction::mint_to(
                token_program_account.key,
                token_mint_account.key,
                token_vault_account.key,
                config_pda,
                &[],
                initial_supply,
            )?,
            &[
                token_mint_account.clone(),
                token_vault_account.clone(),
                config_account.clone(),
            ],
             &[config_signer_seeds],
        )?;

        // 7. CPI: Create Metaplex Metadata Account
        msg!("Creating Metaplex Metadata Account...");
        invoke_signed(
            &metadata_instruction::create_metadata_accounts_v3(
                TOKEN_METADATA_PROGRAM_ID,
                *metadata_account.key,
                *token_mint_account.key,
                *config_pda,
                *payer_creator_account.key,
                *config_pda,
                args.name.clone(), // Clone strings from args reference
                args.symbol.clone(),
                args.metadata_uri.clone(),
                None, 0, true, true, None, None, None,
            ),
            &[
                metadata_account.clone(),
                token_mint_account.clone(),
                config_account.clone(),
                payer_creator_account.clone(),
                system_program_account.clone(),
                rent_sysvar_account.clone(),
                metadata_program_account.clone(),
            ],
            &[config_signer_seeds],
        )?;

        // State Initialization (At the end, using original args)
        msg!("Initializing Bonding Curve Config state at END...");
        let config_data = BondingCurveConfig {
            authority: *payer_creator_account.key,
            state: BondingCurveState::Active,
            token_mint: *token_mint_account.key,
            sol_vault: *sol_vault_account.key,
            token_vault: *token_vault_account.key,
            virtual_sol_reserves: args.initial_virtual_sol_reserves,
            virtual_token_reserves: args.initial_virtual_token_reserves,
            real_sol_reserves: 0,
            total_supply_sold: 0,
            creator: *payer_creator_account.key,
            creation_fee_basis_points: args.creator_fee_basis_points,
            trade_fee_basis_points: args.trade_fee_basis_points,
            market_cap_threshold_sol: args.market_cap_threshold_sol,
            raydium_pool_id: None,
            openbook_market_id: None,
        };
        config_data.serialize(&mut *config_account.data.borrow_mut())?;

        msg!("CreateTokenAndBondingCurve completed successfully.");
        Ok(())
    }

    /// Processes the CreateTokenAndBondingCurve instruction
    fn process_create_token_and_bonding_curve(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: CreateArgs, // Pass args by value here
    ) -> ProgramResult {
        msg!("Processing CreateTokenAndBondingCurve...");

        let account_info_iter = &mut accounts.iter();

        // [X] Account extraction (order matters! Must match client-side order)
        let payer_creator_account = next_account_info(account_info_iter)?;
        let config_account = next_account_info(account_info_iter)?; // PDA
        let token_mint_account = next_account_info(account_info_iter)?; // New mint
        let sol_vault_account = next_account_info(account_info_iter)?; // PDA
        let token_vault_account = next_account_info(account_info_iter)?; // PDA
        let metadata_account = next_account_info(account_info_iter)?; // PDA
        let system_program_account = next_account_info(account_info_iter)?;
        let token_program_account = next_account_info(account_info_iter)?;
        let metadata_program_account = next_account_info(account_info_iter)?; // Metaplex program ID
        let rent_sysvar_account = next_account_info(account_info_iter)?;

        // --- Validation --- 
        msg!("Validating accounts...");

        // [X] Check signers
        if !payer_creator_account.is_signer {
            msg!("Error: Payer/Creator account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // [X] Check writables
        if !config_account.is_writable
            || !token_mint_account.is_writable
            || !sol_vault_account.is_writable
            || !token_vault_account.is_writable
            || !metadata_account.is_writable
            {
            msg!("Error: Required accounts must be writable");
            return Err(ProgramError::InvalidAccountData); // Or a more specific error
        }

        // [X] Check program IDs
        if *system_program_account.key != solana_program::system_program::id() {
            msg!("Error: Invalid system program account");
            return Err(ProgramError::IncorrectProgramId);
        }
        if *token_program_account.key != spl_token::id() {
            msg!("Error: Invalid token program account");
            return Err(ProgramError::IncorrectProgramId);
        }
        // [X] Check Metaplex program ID
        if *metadata_program_account.key != TOKEN_METADATA_PROGRAM_ID {
            msg!("Error: Invalid Metaplex Token Metadata program account");
            return Err(ProgramError::IncorrectProgramId);
        }

        // [X] Check rent sysvar
        if *rent_sysvar_account.key != solana_program::sysvar::rent::id() {
            msg!("Error: Invalid rent sysvar account");
            return Err(ProgramError::IncorrectProgramId);
        }
        let rent = &Rent::from_account_info(rent_sysvar_account)?;

        // --- PDA Derivation & Validation --- 
        msg!("Deriving and validating PDAs...");

        // [X] Derive PDAs
        // Seed strategy: Use mint address for uniqueness. This means the mint account
        // must be created *first* before we can deterministically know the PDA addresses.
        // This complicates the flow slightly compared to seeding with static strings.
        // For now, keeping the previous simple seeds for illustration during build-out.
        // ** IMPORTANT: This needs refinement for production to ensure correct seeding! **
        let (config_pda, config_bump_seed) = Pubkey::find_program_address(
            &[b"config", token_mint_account.key.as_ref()], // Seed with mint address
            program_id,
        );
        let config_signer_seeds = &[b"config", token_mint_account.key.as_ref(), &[config_bump_seed]];

        let (sol_vault_pda, sol_vault_bump_seed) = Pubkey::find_program_address(
            &[b"sol_vault", token_mint_account.key.as_ref()],
            program_id,
        );
        let sol_vault_signer_seeds = &[b"sol_vault", token_mint_account.key.as_ref(), &[sol_vault_bump_seed]];

        let (token_vault_pda, token_vault_bump_seed) = Pubkey::find_program_address(
            &[b"token_vault", token_mint_account.key.as_ref()],
            program_id,
        );
        let token_vault_signer_seeds = &[b"token_vault", token_mint_account.key.as_ref(), &[token_vault_bump_seed]];
        
        // [X] Derive metadata PDA using Metaplex rules
        let (metadata_pda, _metadata_bump_seed) = Pubkey::find_program_address(
            &[
                mpl_token_metadata::pda::PREFIX.as_bytes(),
                TOKEN_METADATA_PROGRAM_ID.as_ref(),
                token_mint_account.key.as_ref(),
            ],
            &TOKEN_METADATA_PROGRAM_ID,
        );

        // [X] Validate derived PDAs against provided accounts
        if config_pda != *config_account.key {
            msg!("Error: Invalid config account PDA");
            return Err(ProgramError::InvalidSeeds);
        }
         if sol_vault_pda != *sol_vault_account.key {
            msg!("Error: Invalid sol vault account PDA");
            return Err(ProgramError::InvalidSeeds);
        }
         if token_vault_pda != *token_vault_account.key {
            msg!("Error: Invalid token vault account PDA");
            return Err(ProgramError::InvalidSeeds);
        }
        // [X] Validate metadata PDA
        if metadata_pda != *metadata_account.key {
             msg!("Error: Invalid metadata account PDA");
            return Err(ProgramError::InvalidSeeds);
        }

        // Call the helper function for CPIs
        Self::create_and_initialize_accounts(
            program_id,
            payer_creator_account,
            config_account,
            token_mint_account,
            sol_vault_account,
            token_vault_account,
            metadata_account,
            system_program_account,
            token_program_account,
            metadata_program_account,
            rent_sysvar_account,
            &config_pda, // Pass derived PDA
            config_signer_seeds,
            sol_vault_signer_seeds,
            token_vault_signer_seeds,
            &rent,
            &args, // Pass original args by reference to helper
        )?;

        msg!("CreateTokenAndBondingCurve completed successfully.");
        Ok(())
    }

    /// Processes the BuyToken instruction
    fn process_buy_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: crate::instruction::BuyArgs,
    ) -> ProgramResult {
        msg!("Processing BuyToken...");
        let account_info_iter = &mut accounts.iter();

        // [X] Account extraction
        let user_signer_account = next_account_info(account_info_iter)?; // Account paying SOL and receiving tokens
        let user_token_account = next_account_info(account_info_iter)?; // User's ATA for the token being bought
        let config_account = next_account_info(account_info_iter)?;    // Curve configuration PDA
        let sol_vault_account = next_account_info(account_info_iter)?;   // PDA SOL vault
        let token_vault_account = next_account_info(account_info_iter)?; // PDA Token vault (owned by config_pda)
        let token_mint_account = next_account_info(account_info_iter)?;  // Mint of the token
        let system_program_account = next_account_info(account_info_iter)?;
        let token_program_account = next_account_info(account_info_iter)?;

        // --- Validation ---
        msg!("Validating BuyToken accounts...");

        // [X] Check signer
        if !user_signer_account.is_signer {
            msg!("Error: User account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // [X] Check writables
        if !user_token_account.is_writable
            || !config_account.is_writable
            || !sol_vault_account.is_writable
            || !token_vault_account.is_writable {
            msg!("Error: Required accounts must be writable");
            return Err(ProgramError::InvalidAccountData);
        }

        // [X] Check program IDs
        if *system_program_account.key != solana_program::system_program::id() {
            msg!("Error: Invalid system program account");
            return Err(ProgramError::IncorrectProgramId);
        }
        if *token_program_account.key != spl_token::id() {
            msg!("Error: Invalid token program account");
            return Err(ProgramError::IncorrectProgramId);
        }

        // [X] Deserialize config state
        let mut config_data = BondingCurveConfig::try_from_slice(&config_account.data.borrow())?;

        // [X] Check curve state
        if config_data.state != BondingCurveState::Active {
            msg!("Error: Bonding curve is not active");
            return Err(TokenLaunchError::InvalidCurveState.into());
        }

        // [X] Check account ownership/keys match config
        if *sol_vault_account.key != config_data.sol_vault {
            msg!("Error: Invalid SOL vault account provided");
            return Err(ProgramError::InvalidAccountData);
        }
        if *token_vault_account.key != config_data.token_vault {
            msg!("Error: Invalid token vault account provided");
            return Err(ProgramError::InvalidAccountData);
        }
        if *token_mint_account.key != config_data.token_mint {
            msg!("Error: Invalid token mint account provided");
            return Err(ProgramError::InvalidAccountData);
        }
        // Check vault owners
        if *sol_vault_account.owner != *program_id {
             msg!("Error: Invalid SOL vault owner");
             return Err(ProgramError::IllegalOwner);
        }
         if *token_vault_account.owner != *token_program_account.key {
             msg!("Error: Invalid Token vault owner");
             return Err(ProgramError::IllegalOwner);
         }

        // --- Calculations ---
        msg!("Calculating buy amounts...");
        if args.sol_amount_in == 0 {
            msg!("Error: SOL amount in must be greater than zero");
            return Err(ProgramError::InvalidArgument);
        }

        // calculate_buy_amount returns only the net token amount out
        let token_amount_out = calculate_buy_amount(
            args.sol_amount_in,
            config_data.virtual_sol_reserves,
            config_data.virtual_token_reserves,
            config_data.trade_fee_basis_points,
        )?;
        
         if token_amount_out == 0 {
             msg!("Error: Calculated token amount out is zero");
             return Err(TokenLaunchError::CalculationError.into()); // Or InvalidAmount
         }

        // --- CPI Calls ---

        // [X] CPI: Transfer SOL from User to SOL Vault PDA
        msg!("Transferring SOL from user to vault...");
        invoke(
            &system_instruction::transfer(
                user_signer_account.key,
                sol_vault_account.key,
                args.sol_amount_in,
            ),
            &[
                user_signer_account.clone(),
                sol_vault_account.clone(),
                system_program_account.clone(),
            ],
        )?; 

        // [X] CPI: Transfer Tokens from Token Vault PDA to User ATA
        msg!("Transferring Tokens from vault to user...");
        // Need Config PDA signer seeds for this
        let (config_pda, config_bump_seed) = Pubkey::find_program_address(
            &[b"config", config_data.token_mint.as_ref()],
            program_id,
        );
        let config_signer_seeds = &[b"config", config_data.token_mint.as_ref(), &[config_bump_seed]];

        // Ensure derived config PDA matches the provided account key
        if config_pda != *config_account.key {
             msg!("Error: Derived config PDA mismatch");
             return Err(ProgramError::InvalidSeeds); // Or another appropriate error
        }

        invoke_signed(
            &token_instruction::transfer(
                token_program_account.key,
                token_vault_account.key, // Source: PDA token vault
                user_token_account.key, // Destination: User's ATA
                &config_pda, // Authority of the token vault
                &[],
                token_amount_out, // User receives net amount
            )?,
            &[
                token_vault_account.clone(),
                user_token_account.clone(),
                config_account.clone(), // Config account (authority)
                token_program_account.clone(),
            ],
            &[config_signer_seeds], // Config PDA signs
        )?; 

        // --- Update State ---
        msg!("Updating bonding curve state...");
        // Increase real SOL reserves by the full amount received (fee stays in vault)
        config_data.real_sol_reserves = config_data.real_sol_reserves
            .checked_add(args.sol_amount_in)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        // Update virtual reserves based on amounts transferred
        config_data.virtual_sol_reserves = config_data.virtual_sol_reserves
            .checked_add(args.sol_amount_in)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        config_data.virtual_token_reserves = config_data.virtual_token_reserves
            .checked_sub(token_amount_out)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        config_data.total_supply_sold = config_data.total_supply_sold
            .checked_add(token_amount_out)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        
        // [X] Migration Check
        // Refresh vault account info to get current lamports AFTER transfer - NO NEED TO RELOAD
        let current_sol_in_vault = sol_vault_account.lamports();
        msg!("Current SOL in vault: {}", current_sol_in_vault);
        msg!("Market cap threshold: {}", config_data.market_cap_threshold_sol);
        if current_sol_in_vault >= config_data.market_cap_threshold_sol {
             msg!("Market cap threshold reached! Locking curve for migration.");
             config_data.state = BondingCurveState::LockedForMigration;
        }

        // [X] Serialize updated state
        config_data.serialize(&mut *config_account.data.borrow_mut())?;

        msg!("BuyToken processed successfully.");
        Ok(())
    }

    /// Processes the SellToken instruction
    fn process_sell_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: crate::instruction::SellArgs, 
    ) -> ProgramResult {
        msg!("Processing SellToken...");
        let account_info_iter = &mut accounts.iter();

        // [X] Account extraction
        let user_signer_account = next_account_info(account_info_iter)?; // Account receiving SOL and sending tokens
        let user_token_account = next_account_info(account_info_iter)?; // User's source ATA for the token being sold
        let config_account = next_account_info(account_info_iter)?;    // Curve configuration PDA
        let sol_vault_account = next_account_info(account_info_iter)?;   // PDA SOL vault
        let token_vault_account = next_account_info(account_info_iter)?; // PDA Token vault
        let token_mint_account = next_account_info(account_info_iter)?;  // Mint of the token
        let system_program_account = next_account_info(account_info_iter)?;
        let token_program_account = next_account_info(account_info_iter)?;

        // --- Validation ---
        msg!("Validating SellToken accounts...");

        // [X] Check signer
        if !user_signer_account.is_signer {
            msg!("Error: User account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // [X] Check writables
        if !user_token_account.is_writable // User ATA needs to be writable for transfer
            || !config_account.is_writable
            || !sol_vault_account.is_writable
            || !token_vault_account.is_writable {
            msg!("Error: Required accounts must be writable");
            return Err(ProgramError::InvalidAccountData);
        }

        // [X] Check program IDs (as in buy)
         if *system_program_account.key != solana_program::system_program::id() {
             msg!("Error: Invalid system program account");
             return Err(ProgramError::IncorrectProgramId);
         }
         if *token_program_account.key != spl_token::id() {
             msg!("Error: Invalid token program account");
             return Err(ProgramError::IncorrectProgramId);
         }

        // [X] Deserialize config state
        let mut config_data = BondingCurveConfig::try_from_slice(&config_account.data.borrow())?;

        // [X] Check curve state
        if config_data.state != BondingCurveState::Active {
            msg!("Error: Bonding curve is not active for selling");
            return Err(TokenLaunchError::InvalidCurveState.into());
        }

        // [X] Check account ownership/keys match config (as in buy)
         if *sol_vault_account.key != config_data.sol_vault {
             msg!("Error: Invalid SOL vault account provided");
             return Err(ProgramError::InvalidAccountData);
         }
         if *token_vault_account.key != config_data.token_vault {
             msg!("Error: Invalid token vault account provided");
             return Err(ProgramError::InvalidAccountData);
         }
         if *token_mint_account.key != config_data.token_mint {
             msg!("Error: Invalid token mint account provided");
             return Err(ProgramError::InvalidAccountData);
         }
         // Check vault owners (as in buy)
          if *sol_vault_account.owner != *program_id {
               msg!("Error: Invalid SOL vault owner");
               return Err(ProgramError::IllegalOwner);
          }
           if *token_vault_account.owner != *token_program_account.key {
               msg!("Error: Invalid Token vault owner");
               return Err(ProgramError::IllegalOwner);
           }

        // --- Calculations ---
        msg!("Calculating sell amounts...");
        if args.token_amount_in == 0 {
             msg!("Error: Token amount in must be greater than zero");
             return Err(ProgramError::InvalidArgument);
        }

        // calculate_sell_amount returns (net_sol_out, gross_sol_out, fee_taken)
        let (sol_amount_out_net, sol_amount_out_gross, _fee_taken) = calculate_sell_amount(
            args.token_amount_in,
            config_data.virtual_sol_reserves,
            config_data.virtual_token_reserves,
            config_data.trade_fee_basis_points,
        )?;

        if sol_amount_out_net == 0 && args.token_amount_in > 0 { // Allow zero net if fee consumes all
            msg!("Warning: Calculated net SOL amount out is zero due to fee"); 
            // Potentially return error if desired, but typically sell is allowed even if fee eats it.
        }

        // [X] Check SOL vault balance
        if sol_vault_account.lamports() < sol_amount_out_net {
            msg!("Error: Insufficient SOL in vault to cover payout");
            return Err(TokenLaunchError::InsufficientFunds.into());
        }

        // --- CPI Calls ---

        // [X] CPI: Transfer Tokens from User ATA to Token Vault PDA
        msg!("Transferring Tokens from user to vault...");
        invoke(
            &token_instruction::transfer(
                token_program_account.key,
                user_token_account.key, // Source: User's ATA
                token_vault_account.key, // Destination: PDA token vault
                user_signer_account.key, // Authority: The user signing the transaction
                &[],
                args.token_amount_in,
            )?,
            &[
                user_token_account.clone(),
                token_vault_account.clone(),
                user_signer_account.clone(), // User is the authority over their ATA
                token_program_account.clone(),
            ],
            // No invoke_signed needed, user is the signer
        )?; 

        // [X] CPI: Transfer SOL from SOL Vault PDA to User Wallet
        msg!("Transferring SOL from vault to user...");
        // Need SOL Vault PDA signer seeds for this
        let (_sol_vault_pda, sol_vault_bump_seed) = Pubkey::find_program_address(
            &[b"sol_vault", config_data.token_mint.as_ref()],
            program_id,
        );
        let sol_vault_signer_seeds = &[b"sol_vault", config_data.token_mint.as_ref(), &[sol_vault_bump_seed]];

        invoke_signed(
            &system_instruction::transfer(
                sol_vault_account.key, // Source: PDA SOL vault
                user_signer_account.key, // Destination: User signer wallet
                sol_amount_out_net, 
            ),
            &[
                sol_vault_account.clone(),
                user_signer_account.clone(),
                system_program_account.clone(),
            ],
            &[sol_vault_signer_seeds], // SOL Vault PDA signs
        )?; 

        // --- Update State ---
        msg!("Updating bonding curve state after sell...");
        // Decrease real SOL reserves by the net amount paid out
        config_data.real_sol_reserves = config_data.real_sol_reserves
            .checked_sub(sol_amount_out_net)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        // Update virtual reserves based on amounts transferred
        config_data.virtual_token_reserves = config_data.virtual_token_reserves
             .checked_add(args.token_amount_in)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        config_data.virtual_sol_reserves = config_data.virtual_sol_reserves
            .checked_sub(sol_amount_out_gross) // Now we have the gross amount!
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;
        config_data.total_supply_sold = config_data.total_supply_sold
             .checked_sub(args.token_amount_in)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?;

        // NOTE: Migration check typically happens on buy, not sell, as buys increase the MC.
        //       We could add it here too if needed, but current logic focuses on buy-side threshold.

        // [X] Serialize updated state
        config_data.serialize(&mut *config_account.data.borrow_mut())?;

        msg!("SellToken processed successfully.");
        Ok(())
    }

    /// Processes the TransferLiquidityAndCompleteMigration instruction (called by backend authority)
    fn process_transfer_liquidity(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: crate::instruction::TransferArgs,
    ) -> ProgramResult {
        msg!("Processing TransferLiquidityAndCompleteMigration...");
        let account_info_iter = &mut accounts.iter();

        // [X] Account extraction
        let authority_signer_account = next_account_info(account_info_iter)?;
        let config_account = next_account_info(account_info_iter)?; // PDA
        let sol_vault_account = next_account_info(account_info_iter)?; // PDA
        let token_vault_account = next_account_info(account_info_iter)?; // PDA
        let destination_sol_account = next_account_info(account_info_iter)?;
        let destination_token_account = next_account_info(account_info_iter)?;
        let system_program_account = next_account_info(account_info_iter)?;
        let token_program_account = next_account_info(account_info_iter)?;

        // [X] Validation
        let mut config_data = BondingCurveConfig::try_from_slice(&config_account.data.borrow())?;

        // [X] Verify authority signer matches config authority
        if !authority_signer_account.is_signer {
            msg!("Error: Authority account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if *authority_signer_account.key != config_data.authority {
             msg!("Error: Signer does not match configured authority");
             return Err(TokenLaunchError::InvalidAuthority.into());
        }

        // Check writables
         if !config_account.is_writable || !sol_vault_account.is_writable || !token_vault_account.is_writable || !destination_sol_account.is_writable || !destination_token_account.is_writable {
            msg!("Error: Required accounts must be writable");
            return Err(ProgramError::InvalidAccountData);
        }
        // Check owners
         if *config_account.owner != *program_id || *sol_vault_account.owner != *program_id || *token_vault_account.owner != *token_program_account.key {
            msg!("Error: Invalid source account owner");
            return Err(ProgramError::IllegalOwner);
        }

        // [X] Assert state is LockedForMigration
        if config_data.state != BondingCurveState::LockedForMigration {
            msg!("Error: Curve is not locked for migration");
            return Err(TokenLaunchError::InvalidCurveState.into());
        }

        // [X] Calculate liquidity amounts to transfer 
        // Example: Transfer 50% of the current SOL in the vault
        // IMPORTANT: Define the token transfer amount logic. Should it be based on 
        // current virtual reserves ratio, or a fixed amount? Let's use ratio for now.
        let total_sol_in_vault = sol_vault_account.lamports();
        let sol_to_transfer = total_sol_in_vault / 2; 

        // Calculate corresponding tokens based on virtual reserves ratio (y/x * sol_to_transfer)
        // Use u128 for intermediate calculations to prevent overflow
        let tokens_to_transfer = (config_data.virtual_token_reserves as u128)
            .checked_mul(sol_to_transfer as u128)
            .ok_or(TokenLaunchError::ArithmeticOverflow)?
            .checked_div(config_data.virtual_sol_reserves as u128)
            .ok_or(TokenLaunchError::ArithmeticOverflow)? as u64;
            
        // TODO: Ensure token_vault has enough tokens_to_transfer (check balance)
        // let token_vault_balance = ... get balance ...
        // if token_vault_balance < tokens_to_transfer { ... error ... }

        // --- CPI Calls ---

        // Derive PDAs needed for signing
        let (_sol_vault_pda, sol_vault_bump_seed) = Pubkey::find_program_address(
            &[b"sol_vault", config_data.token_mint.as_ref()],
            program_id,
        );
        let sol_vault_signer_seeds = &[b"sol_vault", config_data.token_mint.as_ref(), &[sol_vault_bump_seed]];

        let (_token_vault_pda, token_vault_bump_seed) = Pubkey::find_program_address(
            &[b"token_vault", config_data.token_mint.as_ref()],
            program_id,
        );
        let _token_vault_signer_seeds = &[b"token_vault", config_data.token_mint.as_ref(), &[token_vault_bump_seed]];

        // [X] CPI: Transfer SOL from SOL Vault PDA to Destination
        msg!("Transferring SOL liquidity...");
         invoke_signed(
            &system_instruction::transfer(
                sol_vault_account.key, // Source (our vault PDA)
                destination_sol_account.key, // Destination
                sol_to_transfer,
            ),
            &[
                sol_vault_account.clone(),
                destination_sol_account.clone(),
                system_program_account.clone(),
            ],
            &[sol_vault_signer_seeds], // SOL Vault PDA signs
        )?;

        // [X] CPI: Transfer Tokens from Token Vault PDA to Destination
        msg!("Transferring token liquidity...");
        // Re-deriving config PDA/seeds for clarity
        let (config_pda, config_bump_seed) = Pubkey::find_program_address(
            &[b"config", config_data.token_mint.as_ref()],
            program_id,
        );
         let config_signer_seeds = &[b"config", config_data.token_mint.as_ref(), &[config_bump_seed]];
         
         invoke_signed(
            &token_instruction::transfer(
                token_program_account.key,
                token_vault_account.key, 
                destination_token_account.key, 
                &config_pda, // Authority is config PDA
                &[], 
                tokens_to_transfer,
            )?,
            &[
                token_vault_account.clone(),
                destination_token_account.clone(),
                config_account.clone(), // Config account (whose key is config_pda)
                token_program_account.clone(),
            ],
            &[config_signer_seeds], // Config PDA signs
        )?;

        // --- Update State --- 
        msg!("Updating curve state to Migrated...");

        // [X] Update state and store IDs
        config_data.state = BondingCurveState::Migrated;
        config_data.raydium_pool_id = args.raydium_pool_id;
        config_data.openbook_market_id = args.openbook_market_id;

        // Adjust reserves (optional, curve is no longer active)
        config_data.real_sol_reserves = config_data.real_sol_reserves.checked_sub(sol_to_transfer).ok_or(TokenLaunchError::ArithmeticOverflow)?;
        // Virtual reserves maybe zeroed out or left as is?

        // [X] Serialize updated state
        config_data.serialize(&mut *config_account.data.borrow_mut())?;

        // [ ] (Optional) Burn remaining tokens / Close vault accounts - TODO

        msg!("TransferLiquidityAndCompleteMigration completed successfully.");
        Ok(())
    }

    /// Processes the UpdateTradeFee instruction
    fn process_update_trade_fee(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: UpdateTradeFeeArgs,
    ) -> ProgramResult {
        msg!("Processing UpdateTradeFee...");
        let account_info_iter = &mut accounts.iter();

        // Accounts expected:
        // 0. `[signer]` Hardcoded Fee Update Authority account.
        // 1. `[writable]` Config account PDA of the curve to update.
        let authority_signer_account = next_account_info(account_info_iter)?;
        let config_account = next_account_info(account_info_iter)?;

        // --- Validation ---
        msg!("Validating UpdateTradeFee accounts...");

        // Check signer
        if !authority_signer_account.is_signer {
            msg!("Error: Fee Update Authority account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Check writable config
        if !config_account.is_writable {
             msg!("Error: Config account must be writable");
             return Err(ProgramError::InvalidAccountData);
        }

        // Check authority against the hardcoded pubkey
        if *authority_signer_account.key != FEE_UPDATE_AUTHORITY {
             msg!("Error: Signer is not the designated Fee Update Authority");
             // Consider using a more specific custom error if needed
             return Err(TokenLaunchError::InvalidAuthority.into());
        }
        
        // Deserialize config state (still need to read it to modify)
        let mut config_data = BondingCurveConfig::try_from_slice(&config_account.data.borrow())?;

        // --- Update State ---
        msg!("Updating trade fee basis points to: {}", args.new_trade_fee_basis_points);
        if args.new_trade_fee_basis_points > 10000 {
            msg!("Error: New trade fee basis points cannot exceed 10000 (100%)");
            return Err(ProgramError::InvalidArgument);
        }

        config_data.trade_fee_basis_points = args.new_trade_fee_basis_points;

        // Serialize updated state
        config_data.serialize(&mut *config_account.data.borrow_mut())?;

        msg!("UpdateTradeFee processed successfully.");
        Ok(())
    }
}
