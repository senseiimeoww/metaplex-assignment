use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attributes, Plugin, PluginType},
    ID as MPL_CORE_ID,
};

use crate::{constants::*, errors::StakeError, utils::*};

/// Task 1.1: collect accrued rewards while the NFT stays staked and frozen.
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: validated by `load_asset`
    #[account(mut, owner = MPL_CORE_ID @ StakeError::InvalidAsset)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core collection
    #[account(mut, owner = MPL_CORE_ID @ StakeError::InvalidCollection)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer / mint authority
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    #[account(mut, seeds = [REWARD_MINT_SEED, collection.key().as_ref()], bump)]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user
    )]
    pub user_ata: Account<'info, TokenAccount>,

    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimRewards<'info> {
    pub fn handler(&self, bump: u8) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let user_key = self.user.key();
        let collection_key = self.collection.key();

        let core = self.core_program.to_account_info();
        let asset = self.asset.to_account_info();
        let collection = self.collection.to_account_info();
        let user = self.user.to_account_info();
        let update_authority = self.update_authority.to_account_info();
        let system_program = self.system_program.to_account_info();

        load_asset(&asset, &user_key, &collection_key)?;

        let (_, mut attrs, _) =
            fetch_plugin::<BaseAssetV1, Attributes>(&asset, PluginType::Attributes)
                .map_err(|_| error!(StakeError::NotStaked))?;
        require!(is_staked(&attrs.attribute_list), StakeError::NotStaked);

        let reward = calc_rewards(last_claimed(&attrs.attribute_list)?, now)?;
        require!(reward > 0, StakeError::NothingToClaim);

        // Reset the accrual checkpoint; the FreezeDelegate is untouched, so the NFT stays frozen.
        set_attr(
            &mut attrs.attribute_list,
            ATTR_LAST_CLAIMED,
            now.to_string(),
        );

        let seeds: &[&[u8]] = &[UPDATE_AUTHORITY_SEED, collection_key.as_ref(), &[bump]];
        let signer_seeds = &[seeds];

        UpdatePluginV1CpiBuilder::new(&core)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&user)
            .authority(Some(&update_authority))
            .system_program(&system_program)
            .plugin(Plugin::Attributes(attrs))
            .invoke_signed(signer_seeds)?;

        mint_rewards(
            &self.token_program,
            &self.reward_mint,
            &self.user_ata,
            &update_authority,
            signer_seeds,
            reward,
        )
    }
}
