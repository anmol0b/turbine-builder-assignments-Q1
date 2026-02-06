use anchor_lang::prelude::*;
use mpl_core::{
    instructions::CreateV2CpiBuilder,
    types::{
        Attribute, Attributes, BurnDelegate, FreezeDelegate, Plugin, 
        PluginAuthority, PluginAuthorityPair
    },
    ID as CORE_PROGRAM_ID,
};

use crate::{error::MPLXCoreError, state::CollectionAuthority};

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub minter: Signer<'info>,

    #[account(
        mut,
        constraint = asset.data_is_empty() @ MPLXCoreError::AssetAlreadyInitialized
    )]
    pub asset: Signer<'info>,
    /// CHECK: Validated by Core program during CPI call
    #[account(
        mut,
        constraint = collection.owner == &CORE_PROGRAM_ID @ MPLXCoreError::InvalidCollection,
        constraint = !collection.data_is_empty() @ MPLXCoreError::CollectionNotInitialized
    )]
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"collection_authority", collection.key().as_ref()],
        bump = collection_authority.bump,
    )]
    pub collection_authority: Account<'info, CollectionAuthority>,
    /// CHECK: Address constraint ensures correct Core program
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> MintNft<'info> {
    pub fn mint_nft(&mut self) -> Result<()> {
        let attributes = self.build_nft_attributes()?;
        let plugins = self.build_nft_plugins(attributes)?;
        self.execute_mint(plugins)?;
        
        Ok(())
    }

    fn build_nft_attributes(&self) -> Result<Vec<Attribute>> {
        let current_timestamp = Clock::get()?.unix_timestamp;
        
        Ok(vec![
            Attribute {
                key: "Creator".to_string(),
                value: self.collection_authority.creator.to_string(),
            },
            Attribute {
                key: "Minter".to_string(),
                value: self.minter.key().to_string(),
            },
            Attribute {
                key: "Collection".to_string(),
                value: self.collection.key().to_string(),
            },
            Attribute {
                key: "Mint Timestamp".to_string(),
                value: current_timestamp.to_string(),
            },
        ])
    }

    fn build_nft_plugins(&self, attribute_list: Vec<Attribute>) -> Result<Vec<PluginAuthorityPair>> {
        let collection_authority_key = self.collection_authority.key();
        
        Ok(vec![
            PluginAuthorityPair {
                plugin: Plugin::Attributes(Attributes { attribute_list }),
                authority: None,
            },
            PluginAuthorityPair {
                plugin: Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                authority: Some(PluginAuthority::Address {
                    address: collection_authority_key,
                }),
            },
            PluginAuthorityPair {
                plugin: Plugin::BurnDelegate(BurnDelegate {}),
                authority: Some(PluginAuthority::Address {
                    address: collection_authority_key,
                }),
            },
        ])
    }

    fn execute_mint(&self, plugins: Vec<PluginAuthorityPair>) -> Result<()> {
        let collection_key = self.collection.key();
        let bump_seed = &[self.collection_authority.bump];
        
        let signer_seeds = &[
            b"collection_authority".as_ref(),
            collection_key.as_ref(),
            bump_seed,
        ];

        CreateV2CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.collection_authority.to_account_info()))
            .payer(&self.minter.to_account_info())
            .owner(Some(&self.minter.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .name(self.collection_authority.nft_name.clone())
            .uri(self.collection_authority.nft_uri.clone())
            .plugins(plugins)
            .invoke_signed(&[signer_seeds])
            .map_err(|_| error!(MPLXCoreError::MintFailed))?;

        msg!("NFT minted: {} by {}", self.asset.key(), self.minter.key());
        
        Ok(())
    }
}