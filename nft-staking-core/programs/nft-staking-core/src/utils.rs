use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdateCollectionPluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
};

use crate::{constants::*, errors::StakeError};

/// Deserializes the asset and checks owner + collection membership.
pub fn load_asset(asset: &AccountInfo, owner: &Pubkey, collection: &Pubkey) -> Result<BaseAssetV1> {
    let data = asset.try_borrow_data()?;
    let a = BaseAssetV1::from_bytes(&data).map_err(|_| error!(StakeError::InvalidAsset))?;
    require_keys_eq!(a.owner, *owner, StakeError::NotOwner);
    require!(
        matches!(a.update_authority, UpdateAuthority::Collection(c) if c == *collection),
        StakeError::InvalidCollection
    );
    Ok(a)
}

pub fn get_attr<'a>(list: &'a [Attribute], key: &str) -> Option<&'a str> {
    list.iter().find(|a| a.key == key).map(|a| a.value.as_str())
}

pub fn set_attr(list: &mut Vec<Attribute>, key: &str, value: String) {
    match list.iter_mut().find(|a| a.key == key) {
        Some(a) => a.value = value,
        None => list.push(Attribute {
            key: key.to_string(),
            value,
        }),
    }
}

pub fn is_staked(list: &[Attribute]) -> bool {
    get_attr(list, ATTR_STAKED) == Some("true")
}

pub fn last_claimed(list: &[Attribute]) -> Result<i64> {
    get_attr(list, ATTR_LAST_CLAIMED)
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or(error!(StakeError::AttributeNotFound))
}

/// Linear rewards: REWARD_PER_DAY base units per day, accrued per second.
pub fn calc_rewards(last_claimed_at: i64, now: i64) -> Result<u64> {
    let elapsed = now
        .checked_sub(last_claimed_at)
        .ok_or(error!(StakeError::MathOverflow))?;
    require!(elapsed >= 0, StakeError::InvalidTimestamp);
    let reward = (elapsed as u128)
        .checked_mul(REWARD_PER_DAY as u128)
        .ok_or(error!(StakeError::MathOverflow))?
        / SECONDS_PER_DAY as u128;
    u64::try_from(reward).map_err(|_| error!(StakeError::MathOverflow))
}

/// Mints reward tokens to the user's ATA; the update_authority PDA is the mint authority.
pub fn mint_rewards<'info>(
    token_program: &Program<'info, Token>,
    mint: &Account<'info, Mint>,
    to: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    token::mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: mint.to_account_info(),
                to: to.to_account_info(),
                authority: authority.clone(),
            },
            signer_seeds,
        ),
        amount,
    )
}

/// Increments / decrements the `total_staked` Attribute on the Collection account.
pub fn adjust_total_staked<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    increase: bool,
) -> Result<()> {
    let (_, mut attrs, _) =
        fetch_plugin::<BaseCollectionV1, Attributes>(collection, PluginType::Attributes)
            .map_err(|_| error!(StakeError::AttributeNotFound))?;

    let current: u64 = get_attr(&attrs.attribute_list, ATTR_TOTAL_STAKED)
        .and_then(|v| v.parse().ok())
        .ok_or(error!(StakeError::AttributeNotFound))?;

    let updated = if increase {
        current.checked_add(1)
    } else {
        current.checked_sub(1)
    }
    .ok_or(error!(StakeError::MathOverflow))?;

    set_attr(
        &mut attrs.attribute_list,
        ATTR_TOTAL_STAKED,
        updated.to_string(),
    );

    UpdateCollectionPluginV1CpiBuilder::new(core_program)
        .collection(collection)
        .payer(payer)
        .authority(Some(update_authority))
        .system_program(system_program)
        .plugin(Plugin::Attributes(attrs))
        .invoke_signed(signer_seeds)?;

    Ok(())
}
