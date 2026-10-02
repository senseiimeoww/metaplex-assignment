use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod utils;

pub use instructions::*;

// Placeholder. Run `anchor keys sync` to replace it with your real program id.
declare_id!("GrwfMzKxn3P5q2N9v18Gwk2JWiDgeP5sDgCScAaJzPG7");

#[program]
pub mod nft_staking_core {
    use super::*;

    /// Creates the collection with its staking counter and reward mint.
    pub fn initialize(ctx: Context<Initialize>, name: String, uri: String) -> Result<()> {
        ctx.accounts.handler(name, uri)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.handler(ctx.bumps.update_authority)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.handler(ctx.bumps.update_authority)
    }

    /// Task 1.1
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.handler(ctx.bumps.update_authority)
    }

    /// Task 1.2
    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.handler(ctx.bumps.update_authority)
    }
}
