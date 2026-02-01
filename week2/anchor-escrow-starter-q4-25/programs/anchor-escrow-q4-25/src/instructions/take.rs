#![allow(unused_imports)]

use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::Escrow;

#[derive(Accounts)]
pub struct Take<'info> {
    //  TODO: Implement Take Accounts
    #[account(mut)]
    pub taker: Signer<'info>,
    pub maker: SystemAccount<'info>,
    #[account(
        mint::token_program = token_program,
        constraint = mint_a.key() == escrow.mint_a
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mint::token_program = token_program,
        constraint = mint_b.key() == escrow.mint_b
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer=taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        close = taker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    //  TODO: Implement Take Instruction
    //  Includes Deposit, Withdraw and Close Vault
    pub fn deposit(&mut self) -> Result<()> {
        let amount = self.escrow.receive;
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                mint: self.mint_b.to_account_info(),
                from: self.taker_ata_b.to_account_info(),
                to: self.maker_ata_b.to_account_info(),
                authority: self.taker.to_account_info(),
            },
        );

        transfer_checked(cpi_ctx, amount, self.mint_b.decimals)
    }

    pub fn withdraw(&mut self) -> Result<()> {
        let amount = self.vault.amount;
        let maker_key = self.maker.key();
        let seed = self.escrow.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            maker_key.as_ref(),
            seed.as_ref(),
            &[self.escrow.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                mint: self.mint_a.to_account_info(),
                from: self.vault.to_account_info(),
                to: self.taker_ata_a.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        );

        transfer_checked(cpi_ctx, amount, self.mint_a.decimals)
    }

    pub fn close_vault(&mut self) -> Result<()> {
        let maker_key = self.maker.key();
        let seed = self.escrow.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            maker_key.as_ref(),
            seed.as_ref(),
            &[self.escrow.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.taker.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        );

        close_account(cpi_ctx)
    }
}
