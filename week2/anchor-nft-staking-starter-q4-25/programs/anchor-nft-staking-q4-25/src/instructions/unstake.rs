use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin, PluginType},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    errors::StakeError,
    state::{StakeAccount, StakeConfig, UserAccount},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    //TODO
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: asset is verified by the core program
    #[account(
        mut,
        constraint = collection.owner == &CORE_PROGRAM_ID @ StakeError::InvalidCollection,
        constraint = !collection.data_is_empty() @ StakeError::CollectionNotInitialized
    )]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: collection is verified by the core program
    #[account(
        mut,
        constraint = asset.owner == &CORE_PROGRAM_ID @ StakeError::InvalidCollection,
        constraint = !asset.data_is_empty() @ StakeError::CollectionNotInitialized
    )]
    pub asset: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds= [b"user", user.key().as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(
        seeds = [b"config".as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(
        mut,
        close = user,
        constraint = stake_account.owner == user.key(),
        seeds = [b"stake", config.key().as_ref(), asset.key().as_ref()],
        bump = stake_account.bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
     /// CHECK: verified by the metaplex core
    #[account(address=CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&mut self) -> Result<()> {
        let time_elapsed =
            ((Clock::get()?.unix_timestamp - self.stake_account.staked_at) / 86400) as u32;
        require!(
            time_elapsed >= self.config.freeze_period,
            StakeError::FreezePeriodNotPassed
        );

        self.user_account.points = self
            .user_account
            .points
            .checked_add(time_elapsed * self.config.points_per_stake as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let signers_seeds: &[&[&[u8]]] = &[&[
            b"stake",
            &self.config.key().to_bytes(),
            &self.asset.key().to_bytes(),
            &[self.stake_account.bump],
        ]];
        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.stake_account.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signers_seeds)?;

        RemovePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(None)
            .system_program(&self.system_program.to_account_info())
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        self.user_account.amount_staked = self
            .user_account
            .amount_staked
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }
}
