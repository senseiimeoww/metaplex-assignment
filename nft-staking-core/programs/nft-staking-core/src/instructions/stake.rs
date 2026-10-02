use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, BurnDelegate, FreezeDelegate, Plugin, PluginAuthority, PluginType,
    },
    ID as MPL_CORE_ID,
};

use crate::{constants::*, errors::StakeError, utils::*};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: validated by `load_asset`
    #[account(mut, owner = MPL_CORE_ID @ StakeError::InvalidAsset)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core collection
    #[account(mut, owner = MPL_CORE_ID @ StakeError::InvalidCollection)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
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

        // 1) Attributes on the asset (authority-managed -> PDA signs)
        match fetch_plugin::<BaseAssetV1, Attributes>(&asset, PluginType::Attributes) {
            Ok((_, mut attrs, _)) => {
                require!(!is_staked(&attrs.attribute_list), StakeError::AlreadyStaked);
                set_attr(&mut attrs.attribute_list, ATTR_STAKED, "true".to_string());
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
            }
            Err(_) => {
                AddPluginV1CpiBuilder::new(&core)
                    .asset(&asset)
                    .collection(Some(&collection))
                    .payer(&user)
                    .authority(Some(&update_authority))
                    .system_program(&system_program)
                    .plugin(Plugin::Attributes(Attributes {
                        attribute_list: vec![
                            Attribute {
                                key: ATTR_STAKED.to_string(),
                                value: "true".to_string(),
                            },
                            Attribute {
                                key: ATTR_LAST_CLAIMED.to_string(),
                                value: now.to_string(),
                            },
                        ],
                    }))
                    .init_authority(PluginAuthority::UpdateAuthority)
                    .invoke_signed(signer_seeds)?;
            }
        }

        // 2) FreezeDelegate (owner-managed -> owner signs), PDA is the delegate
        AddPluginV1CpiBuilder::new(&core)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&user)
            .authority(Some(&user))
            .system_program(&system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::Address {
                address: update_authority.key(),
            })
            .invoke()?;

        // 3) BurnDelegate so the program can burn on the user's behalf (Task 1.2)
        AddPluginV1CpiBuilder::new(&core)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&user)
            .authority(Some(&user))
            .system_program(&system_program)
            .plugin(Plugin::BurnDelegate(BurnDelegate {}))
            .init_authority(PluginAuthority::Address {
                address: update_authority.key(),
            })
            .invoke()?;

        // 4) Task 1.3: total_staked += 1
        adjust_total_staked(
            &core,
            &collection,
            &user,
            &update_authority,
            &system_program,
            signer_seeds,
            true,
        )?;

        Ok(())
    }
}
