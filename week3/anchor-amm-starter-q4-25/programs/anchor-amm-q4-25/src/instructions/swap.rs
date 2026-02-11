use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
        pub user: Signer<'info>,
        pub mint_x: Box<Account<'info, Mint>>,
        pub mint_y: Box<Account<'info, Mint>>,
        #[account(
            has_one = mint_x,
            has_one = mint_y,
            seeds = [b"config", config.seed.to_le_bytes().as_ref()],
            bump = config.config_bump,
        )]
        pub config: Box<Account<'info, Config>>,
        #[account(
            seeds = [b"lp", config.key().as_ref()],
            bump = config.lp_bump,
        )]
        pub mint_lp: Box<Account<'info, Mint>>,
        #[account(
            mut,
            associated_token::mint = mint_x,
            associated_token::authority = config,
            associated_token::token_program = token_program

        )]
        pub vault_x: Box<Account<'info, TokenAccount>>,
        #[account(
            mut,
            associated_token::mint = mint_y,
            associated_token::authority = config,
            associated_token::token_program = token_program

        )]
        pub vault_y: Box<Account<'info, TokenAccount>>,
        #[account(
            init_if_needed,
            payer = user,
            associated_token::mint = mint_x,
            associated_token::authority = user,
            associated_token::token_program = token_program

        )]
        pub user_x: Box<Account<'info, TokenAccount>>,
        #[account(
            init_if_needed,
            payer = user,
            associated_token::mint = mint_y,
            associated_token::authority = user,
            associated_token::token_program = token_program

        )]
        pub user_y: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
        pub associated_token_program: Program<'info, AssociatedToken>,
        pub system_program: Program<'info, System>,

}
impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount_in != 0, AmmError::InvalidAmount);
        require!(
            self.vault_x.amount > 0 && self.vault_y.amount > 0,
            AmmError::NoLiquidityInPool
        );

        // Check user has sufficient balance
        if is_x {
            require!(
                self.user_x.amount >= amount_in,
                AmmError::InsufficientBalance
            );
        } else {
            require!(
                self.user_y.amount >= amount_in,
                AmmError::InsufficientBalance
            );
        }

        let mut product_curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.config.fee,
            Some(6u8), // 6 decimals precision
        )
        .map_err(Into::<AmmError>::into)?;

        let swap_result = if is_x {
            product_curve
                .swap(LiquidityPair::X, amount_in, min_amount_out)
                .map_err(Into::<AmmError>::into)?
        } else {
            product_curve
                .swap(LiquidityPair::Y, amount_in, min_amount_out)
                .map_err(Into::<AmmError>::into)?
        };

        require!(swap_result.deposit != 0, AmmError::InvalidAmount);
        require!(swap_result.withdraw != 0, AmmError::InvalidAmount);

        // Deposit input tokens (user -> vault)
        self.deposit_tokens(is_x, swap_result.deposit)?;

        // Withdraw output tokens (vault -> user)
        // When swapping X for Y, we withdraw Y (opposite token)
        self.withdraw_tokens(!is_x, swap_result.withdraw)?;

        Ok(())
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            )
        } else {
            (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            )
        };

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };
        let ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(ctx, amount)
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        // Check vault has sufficient balance
        if is_x {
            require!(
                self.vault_x.amount >= amount,
                AmmError::InsufficientBalance
            );
        } else {
            require!(
                self.vault_y.amount >= amount,
                AmmError::InsufficientBalance
            );
        }

        let (from, to) = if is_x {
            (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            )
        } else {
            (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            )
        };

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.config.to_account_info(),
        };
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];
        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(ctx, amount)
    }
}