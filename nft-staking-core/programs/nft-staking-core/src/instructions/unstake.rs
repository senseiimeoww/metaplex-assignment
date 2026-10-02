use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attributes, FreezeDelegate, Plugin, PluginType},
    ID as MPL_CORE_ID,
};

use crate::{constants::*, errors::StakeError, utils::*};

#[derive(Accounts)]
pub struct Unstake<'info> {
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

impl<'info> Unstake<'info> {
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

        let seeds: &[&[u8]] = &[UPDATE_AUTHORITY_SEED, collection_key.as_ref(), &[bump]];
        let signer_seeds = &[seeds];

        // Pending rewards + mark as unstaked
        let (_, mut attrs, _) =
            fetch_plugin::<BaseAssetV1, Attributes>(&asset, PluginType::Attributes)
                .map_err(|_| error!(StakeError::NotStaked))?;
        require!(is_staked(&attrs.attribute_list), StakeError::NotStaked);

        let pending = calc_rewards(last_claimed(&attrs.attribute_list)?, now)?;

        set_attr(&mut attrs.attribute_list, ATTR_STAKED, "false".to_string());
        set_attr(
            &mut attrs.attribute_list,
            ATTR_LAST_CLAIMED,
            now.to_string(),
        );

        UpdatePluginV1CpiBuilder::new(&core)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&user)
            .authority(Some(&update_authority))
            .system_program(&system_program)
            .plugin(Plugin::Attributes(attrs))
            .invoke_signed(signer_seeds)?;

        // Unfreeze (PDA is the freeze delegate)
        UpdatePluginV1CpiBuilder::new(&core)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&user)
            .authority(Some(&update_authority))
            .system_program(&system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;

        // Remove delegates (owner signs)
        for plugin_type in [PluginType::FreezeDelegate, PluginType::BurnDelegate] {
            RemovePluginV1CpiBuilder::new(&core)
                .asset(&asset)
                .collection(Some(&collection))
                .payer(&user)
                .authority(Some(&user))
                .system_program(&system_program)
                .plugin_type(plugin_type)
                .invoke()?;
        }

        // total_staked -= 1
        adjust_total_staked(
            &core,
            &collection,
            &user,
            &update_authority,
            &system_program,
            signer_seeds,
            false,
        )?;

        mint_rewards(
            &self.token_program,
            &self.reward_mint,
            &self.user_ata,
            &update_authority,
            signer_seeds,
            pending,
        )
    }
}
