use anchor_lang::prelude::error_code;

#[error_code]
pub enum MPLXCoreError {
    #[msg("The creator list is full.")]
    CreatorListFull,
    #[msg("The creator is already in the list.")]
    CreatorAlreadyWhitelisted,
    #[msg("The payer is not the program's upgrade authority.")]
    NotAuthorized,
    #[msg("The collection has already been initialized.")]
    CollectionAlreadyInitialized,
    #[msg("The asset has already been initialized.")]
    AssetAlreadyInitialized,
    #[msg("The collection is not initialized.")]
    CollectionNotInitialized,
    #[msg("The collection is invalid.")]
    InvalidCollection,
    #[msg("Invalid asset account.")]
    InvalidAsset,
    #[msg("Asset not initialized.")]
    AssetNotInitialized,
    #[msg("Collection mismatch.")]
    CollectionMismatch,
    #[msg("Freeze failed.")]
    FreezeFailed,
    #[msg("Thaw failed.")]
    ThawFailed,
    #[msg("Invalid name.")]
    InvalidName,
    #[msg("Name too long.")]
    NameTooLong,
    #[msg("Update failed.")]
    UpdateFailed,
    #[msg("Mint failed.")]
    MintFailed,
}