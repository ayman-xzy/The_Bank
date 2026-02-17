#![allow(unexpected_cfgs)]
use anchor_lang::{prelude::*, system_program::{transfer, Transfer}};

declare_id!("8fegMuK3ZrhxZNBh9woL1Nx8aiqHUpwscQQhfAaSvA2C");

#[program]
pub mod the_bank {
    use super::*;

    // Initializing a new vault for the user.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        
        ctx.accounts.initialize(&ctx.bumps)?; // Sets up vault state with PDA bumps.

        // initialization event with the user's public key.
        emit!(InitializeEvent {
            user: ctx.accounts.user.key(),
        });

        Ok(())
    }

    // Deposits SOL into the vault from the user's account.
    pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
        // Transfers the specified amount of SOL to the vault.
        ctx.accounts.deposit(amount)?;

        // Logs the deposit event with user and amount details.
        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }

    // Withdraws SOL from the vault back to the user's account.
    pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
        // Transfers the specified amount of SOL from the vault to the user.
        ctx.accounts.withdraw(amount)?;

        // Logs the withdrawal event with user and amount details.
        emit!(WithdrawEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }

    // Closes the vault, transferring all remaining SOL to the user and cleaning up.
    pub fn close(ctx: Context<CloseAccount>) -> Result<()> {
        // Captures the current balance of the vault in lamports (SOL's smallest unit).
        let amount = ctx.accounts.vault.lamports();

        // Closes the vault and transfers funds to the user.
        ctx.accounts.close()?;

        // Logs the closure event with user and final amount withdrawn.
        emit!(CloseEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
}

// Defines the accounts required for initializing the vault.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // The user who signs the transaction to create the vault.

    #[account(
        init, // Creates a new account for the vault state.
        payer = user, // User pays the rent for this account.
        seeds = [b"state", user.key().as_ref()], // PDA seeds for uniqueness.
        bump, // Bump seed for PDA derivation.
        space = VaultState::INIT_SPACE, // Space allocated for the vault state data.
    )]
    pub vault_state: Account<'info, VaultState>, // Stores vault configuration.

    #[account(
        seeds = [b"vault", vault_state.key().as_ref()], // PDA seeds for the vault.
        bump, // Bump seed for the vault PDA.
    )]
    pub vault: SystemAccount<'info>, // Holds the SOL deposited into the vault.

    pub system_program: Program<'info, System> // Required for account creation and transfers.
}

// Methods for the Initialize struct.
impl<'info> Initialize<'info> {
    // Sets up the vault state with bump seeds for PDAs.
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.vault_state.vault_bump = bumps.vault; // Stores vault PDA bump.
        self.vault_state.state_bump = bumps.vault_state; // Stores state PDA bump.
        Ok(())
    }
}

// Defines the accounts for deposit and withdraw operations.
#[derive(Accounts)]
pub struct Payment<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // The user performing the deposit or withdrawal.

    #[account(
        seeds = [b"state", user.key().as_ref()], // Matches the vault state PDA.
        bump = vault_state.state_bump // Ensures correct PDA with stored bump.
    )]
    pub vault_state: Account<'info, VaultState>, // Accesses vault configuration.

    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()], // Matches the vault PDA.
        bump = vault_state.vault_bump // Ensures correct PDA with stored bump.
    )]
    pub vault: SystemAccount<'info>, // The account holding the SOL.

    pub system_program: Program<'info, System> // Used for SOL transfers.
}

// Methods for the Payment struct.
impl<'info> Payment<'info> {
    // Deposits SOL from the user to the vault using a CPI (Cross-Program Invocation).
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info(); // System program for transfer.
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(), // User's account as the source.
            to: self.vault.to_account_info(), // Vault as the destination.
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts); // Context for the transfer.
        transfer(cpi_ctx, amount) // Executes the SOL transfer.
    }

    // Withdraws SOL from the vault to the user, signed by the vault PDA.
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info(); // System program for transfer.
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(), // Vault as the source.
            to: self.user.to_account_info(), // User's account as the destination.
        };
        // Seeds for PDA signing authority.
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]]; // Array of seeds for signing.
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds); // Signed context.
        transfer(cpi_ctx, amount) // Executes the SOL transfer.
    }
}

// Defines the accounts required to close the vault.
#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // The user closing the vault.

    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()], // Matches the vault state PDA.
        bump = vault_state.state_bump, // Ensures correct PDA.
        close = user // Closes the state account, refunding rent to the user.
    )]
    pub vault_state: Account<'info, VaultState>, // Vault configuration to be closed.

    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()], // Matches the vault PDA.
        bump = vault_state.vault_bump, // Ensures correct PDA.
    )]
    pub vault: SystemAccount<'info>, // Vault account to be drained.

    pub system_program: Program<'info, System>, // Required for transfers and closure.
}

// Methods for the CloseAccount struct.
impl<'info> CloseAccount<'info> {
    // Transfers all SOL from the vault to the user and closes the account.
    pub fn close(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info(); // System program for transfer.
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(), // Vault as the source.
            to: self.user.to_account_info(), // User's account as the destination.
        };
        // Seeds for PDA signing authority.
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]]; // Array of seeds for signing.
        let cpi_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds); // Signed context.
        transfer(cpi_context, self.vault.lamports()) // Transfers all SOL, effectively closing the vault.
    }
}

// Data structure to store vault state information.
#[account]
pub struct VaultState {
    pub vault_bump: u8, // Bump seed for the vault PDA.
    pub state_bump: u8, // Bump seed for the vault state PDA.
}

// Defines the space required for VaultState.
impl Space for VaultState {
    const INIT_SPACE: usize = 8 + 1 + 1; // 8 bytes for discriminator + 1 for vault_bump + 1 for state_bump.
}

// Events for logging actions on the blockchain.

// Logs when a vault is initialized.
#[event]
pub struct InitializeEvent {
    pub user: Pubkey, // Public key of the user who initialized the vault.
}

// Logs when SOL is deposited into the vault.
#[event]
pub struct DepositEvent {
    pub user: Pubkey, // Public key of the user who deposited.
    pub amount: u64, // Amount of SOL deposited.
}

// Logs when SOL is withdrawn from the vault.
#[event]
pub struct WithdrawEvent {
    pub user: Pubkey, // Public key of the user who withdrew.
    pub amount: u64, // Amount of SOL withdrawn.
}

// Logs when the vault is closed.
#[event]
pub struct CloseEvent {
    pub user: Pubkey, // Public key of the user who closed the vault.
    pub amount: u64, // Amount of SOL transferred during closure.
}