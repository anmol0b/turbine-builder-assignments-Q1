use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    //TODO
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
            mut,
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
        #[account(
            mut,
            associated_token::mint = mint_lp,
            associated_token::authority = user,
            associated_token::token_program = token_program
        )]
        pub user_lp: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
        pub associated_token_program: Program<'info, AssociatedToken>,
        pub system_program: Program<'info, System>,

}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "burn"
        min_x: u64,  // Minimum amount of token X that the user wants to receive
        min_y: u64,  // Minimum amount of token Y that the user wants to receive
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);
        require!(
            self.user_lp.amount >= amount,
            AmmError::InsufficientBalance
        );
        require!(
            self.mint_lp.supply > 0 && self.vault_x.amount > 0 && self.vault_y.amount > 0,
            AmmError::NoLiquidityInPool
        );

        let (x, y) = {
            let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                self.vault_x.amount,
                self.vault_y.amount,
                self.mint_lp.supply,
                amount,
                6u32, // 6 decimals precision
            )
            .map_err(Into::<AmmError>::into)?;
            (amounts.x, amounts.y)
        };

        require!(x >= min_x && y >= min_y, AmmError::SlippageExceeded);
        require!(self.vault_x.amount >= x, AmmError::NoLiquidityInPool);
        require!(self.vault_y.amount >= y, AmmError::NoLiquidityInPool);

        self.withdraw_tokens(true, x)?;
        self.withdraw_tokens(false, y)?;
        self.burn_lp_tokens(amount)
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
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

    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let ctx = CpiContext::new(cpi_program, cpi_accounts);

        burn(ctx, amount)
    }
}
