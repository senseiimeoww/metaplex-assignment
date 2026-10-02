use anchor_lang::prelude::*;

#[error_code]
pub enum StakeError {
    #[msg("Account is not a valid Core asset")]
    InvalidAsset,
    #[msg("Asset does not belong to this collection")]
    InvalidCollection,
    #[msg("Signer does not own this asset")]
    NotOwner,
    #[msg("NFT is already staked")]
    AlreadyStaked,
    #[msg("NFT is not staked")]
    NotStaked,
    #[msg("Required attribute is missing or malformed")]
    AttributeNotFound,
    #[msg("No rewards to claim yet")]
    NothingToClaim,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
