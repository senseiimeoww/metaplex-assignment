use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use mpl_core::{
    instructions::CreateCollectionV2CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginAuthority, PluginAuthorityPair},
    ID as MPL_CORE_ID,
};

use crate::constants::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// New collection keypair; must sign.
    #[account(mut)]
    pub collection: Signer<'info>,

    /// CHECK: PDA. Collection update authority, freeze/burn delegate and reward mint authority.
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = REWARD_DECIMALS,
        mint::authority = update_authority,
        seeds = [REWARD_MINT_SEED, collection.key().as_ref()],
        bump
    )]
    pub reward_mint: Account<'info, Mint>,

    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(&self, name: String, uri: String) -> Result<()> {
        let core = self.core_program.to_account_info();
        let collection = self.collection.to_account_info();
        let admin = self.admin.to_account_info();
        let update_authority = self.update_authority.to_account_info();
        let system_program = self.system_program.to_account_info();

        CreateCollectionV2CpiBuilder::new(&core)
            .collection(&collection)
            .update_authority(Some(&update_authority))
            .payer(&admin)
            .system_program(&system_program)
            .name(name)
            .uri(uri)
            // Task 1.3: total_staked counter on the Collection
            .plugins(vec![PluginAuthorityPair {
                plugin: Plugin::Attributes(Attributes {
                    attribute_list: vec![Attribute {
                        key: ATTR_TOTAL_STAKED.to_string(),
                        value: "0".to_string(),
                    }],
                }),
                authority: Some(PluginAuthority::UpdateAuthority),
            }])
            .invoke()?;

        Ok(())
    }
}
