use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

declare_id!("4XeRRycAsykrjrYYVwZLqtC4FCurZzwxnwPU2qonv3Ui");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;

        vault_state.authority = ctx.accounts.authority.key();
        vault_state.mint = ctx.accounts.mint.key();
        vault_state.deposited = 0;
        vault_state.bump = ctx.bumps.vault_state;
        vault_state.vault_bump = ctx.bumps.vault_token_account;

        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        let transfer_accounts = TransferChecked {
            from: ctx.accounts.authority_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        );

        transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

        let vault_state = &mut ctx.accounts.vault_state;
        vault_state.deposited = vault_state
            .deposited
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            ctx.accounts.vault_state.deposited >= amount,
            VaultError::InsufficientFunds
        );

        let authority = ctx.accounts.vault_state.authority;
        let mint = ctx.accounts.vault_state.mint;
        let bump = ctx.accounts.vault_state.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[b"state", authority.as_ref(), mint.as_ref(), &[bump]]];

        let transfer_accounts = TransferChecked {
            from: ctx.accounts.vault_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.vault_state.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        );

        transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

        let vault_state = &mut ctx.accounts.vault_state;
        vault_state.deposited = vault_state
            .deposited
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [b"state", authority.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = vault_state,
        seeds = [b"vault", authority.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = authority,
        has_one = mint,
        seeds = [b"state", authority.key().as_ref(), mint.key().as_ref()],
        bump = vault_state.bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = authority
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_state,
        seeds = [b"vault", authority.key().as_ref(), mint.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = authority,
        has_one = mint,
        seeds = [b"state", authority.key().as_ref(), mint.key().as_ref()],
        bump = vault_state.bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = authority
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_state,
        seeds = [b"vault", authority.key().as_ref(), mint.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub deposited: u64,
    pub bump: u8,
    pub vault_bump: u8,
}

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Vault does not have enough deposited tokens")]
    InsufficientFunds,
    #[msg("Token amount arithmetic overflowed")]
    MathOverflow,
}
