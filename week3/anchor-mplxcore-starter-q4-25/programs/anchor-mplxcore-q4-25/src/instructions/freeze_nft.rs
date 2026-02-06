use anchor_lang::prelude::*;
use mpl_core::{
    instructions::UpdatePluginV1CpiBuilder,
    types::{FreezeDelegate, Plugin},
    ID as CORE_PROGRAM_ID,
};

use crate::{error::MPLXCoreError, state::CollectionAuthority};

#[derive(Accounts)]
pub struct FreezeNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Validated by Core program during CPI call
    #[account(
        mut,
        constraint = *asset.owner == CORE_PROGRAM_ID @ MPLXCoreError::InvalidAsset,
        constraint = !asset.data_is_empty() @ MPLXCoreError::AssetNotInitialized,
    )]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: Validated by Core program during CPI call
    #[account(
        mut,
        constraint = *collection.owner == CORE_PROGRAM_ID @ MPLXCoreError::InvalidCollection,
        constraint = !collection.data_is_empty() @ MPLXCoreError::CollectionNotInitialized
    )]
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"collection_authority", collection.key().as_ref()],
        bump = collection_authority.bump,
        constraint = collection_authority.creator == authority.key() @ MPLXCoreError::NotAuthorized,
        constraint = collection_authority.collection == collection.key() @ MPLXCoreError::CollectionMismatch
    )]
    pub collection_authority: Account<'info, CollectionAuthority>,
    /// CHECK: Address constraint ensures correct Core program
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> FreezeNft<'info> {
    pub fn freeze_nft(&mut self) -> Result<()> {
        self.validate_freeze_eligibility()?;
        self.execute_freeze()?;
        
        Ok(())
    }

    fn validate_freeze_eligibility(&self) -> Result<()> {
        Ok(())
    }

    fn execute_freeze(&self) -> Result<()> {
        let collection_key = self.collection.key();
        
        let signer_seeds = &[
            b"collection_authority".as_ref(),
            collection_key.as_ref(),
            &[self.collection_authority.bump],
        ];

        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .payer(&self.authority.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.collection_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .invoke_signed(&[signer_seeds])
            .map_err(|_| error!(MPLXCoreError::FreezeFailed))?;

        msg!("NFT frozen: {}", self.asset.key());
        
        Ok(())
    }
}