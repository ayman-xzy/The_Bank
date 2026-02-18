#![allow(unexpected_cfgs)]
use anchor_lang::{prelude::*, system_program::{transfer, Transfer}};

declare_id!("8fegMuK3ZrhxZNBh9woL1Nx8aiqHUpwscQQhfAaSvA2C");

#[program]
pub mod the_bank {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        
        ctx.accounts.initialize(&ctx.bumps)?; 

        emit!(InitializeEvent {
            user: ctx.accounts.user.key(),
        });

        Ok(())
    }
    pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)?;

        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
    pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)?;

        emit!(WithdrawEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
    pub fn close(ctx: Context<CloseAccount>) -> Result<()> {
        let amount = ctx.accounts.vault.lamports();

        ctx.accounts.close()?;

        emit!(CloseEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub vault_bump: u8, 
    pub state_bump: u8, 
    pub deposit_timestamp: i64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>, 

    #[account(
        init, 
        payer = user,
        seeds = [b"state", user.key().as_ref()], 
        bump, 
        space = 8 + VaultState::INIT_SPACE, 
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        seeds = [b"vault", vault_state.key().as_ref()], 
        bump, 
    )]
    pub vault: SystemAccount<'info>, 
    pub system_program: Program<'info, System> 
}

impl<'info> Initialize<'info> {
   
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.vault_state.vault_bump = bumps.vault; 
        self.vault_state.state_bump = bumps.vault_state; 
        self.vault_state.deposit_timestamp = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Payment<'info> {
    #[account(mut)]
    pub user: Signer<'info>, 

    #[account(
        seeds = [b"state", user.key().as_ref()], 
        bump = vault_state.state_bump 
    )]
    pub vault_state: Account<'info, VaultState>, 

    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()], 
        bump = vault_state.vault_bump 
    )]
    pub vault: SystemAccount<'info>, 
    pub system_program: Program<'info, System>
}

impl<'info> Payment<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info(); 
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(), 
            to: self.vault.to_account_info(), 
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts); 
        transfer(cpi_ctx, amount)?;

        self.vault_state.deposit_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<()> {

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 2 days = 172800 seconds
        require!(
            current_time >= self.vault_state.deposit_timestamp + 172800,
            ErrorCode::WithdrawalTooEarly
        );

        let cpi_program = self.system_program.to_account_info(); 
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(), 
            to: self.user.to_account_info(),
        };
        
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]]; 
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds); // Signed context.
        transfer(cpi_ctx, amount) 
    }
}


#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(mut)]
    pub user: Signer<'info>, 

    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()], 
        bump = vault_state.state_bump, 
        close = user 
    )]
    pub vault_state: Account<'info, VaultState>, 

    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump, 
    )]
    pub vault: SystemAccount<'info>, 
    pub system_program: Program<'info, System>, 
}

impl<'info> CloseAccount<'info> {
    
    pub fn close(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info(); 
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(), 
        };
       
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]]; 
        let cpi_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds); // Signed context.
        transfer(cpi_context, self.vault.lamports()) 
    }
}


#[event]
pub struct InitializeEvent {
    pub user: Pubkey, 
}
#[event]
pub struct DepositEvent {
    pub user: Pubkey, 
    pub amount: u64, 
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey, 
    pub amount: u64, 
}

#[event]
pub struct CloseEvent {
    pub user: Pubkey, 
    pub amount: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Withdrawal is locked for 2 days after deposit")]
    WithdrawalTooEarly,
}
