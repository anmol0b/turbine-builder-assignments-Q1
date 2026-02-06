use anchor_lang::prelude::*;
use mpl_core::{instructions::UpdateV2CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::{error::MPLXCoreError, state::CollectionAuthority};

#[derive(Accounts)]
pub struct UpdateNft<'info> {
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

impl<'info> UpdateNft<'info> {
    pub fn update_nft(&mut self, new_name: String) -> Result<()> {
        self.validate_update_params(&new_name)?;
        self.execute_update(new_name)?;
        
        Ok(())
    }

    fn validate_update_params(&self, new_name: &str) -> Result<()> {
        require!(!new_name.is_empty(), MPLXCoreError::InvalidName);
        require!(new_name.len() <= 32, MPLXCoreError::NameTooLong);
        
        Ok(())
    }

    fn execute_update(&self, new_name: String) -> Result<()> {
        let collection_key = self.collection.key();
        let bump_seed = &[self.collection_authority.bump];
        
        let signer_seeds = &[
            b"collection_authority".as_ref(),
            collection_key.as_ref(),
            bump_seed,
        ];

        UpdateV2CpiBuilder::new(&self.core_program.to_account_info())
            .payer(&self.authority.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.collection_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .new_name(new_name.clone())
            .invoke_signed(&[signer_seeds])
            .map_err(|_| error!(MPLXCoreError::UpdateFailed))?;

        msg!("NFT updated: {} -> {}", self.asset.key(), new_name);
        
        Ok(())
    }
}