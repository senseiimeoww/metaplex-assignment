// PDA seeds
pub const UPDATE_AUTHORITY_SEED: &[u8] = b"update_authority";
pub const REWARD_MINT_SEED: &[u8] = b"reward_mint";

// Attribute keys
pub const ATTR_STAKED: &str = "staked";
pub const ATTR_LAST_CLAIMED: &str = "last_claimed_at";
pub const ATTR_TOTAL_STAKED: &str = "total_staked";

// Rewards (reward mint has 6 decimals)
pub const REWARD_DECIMALS: u8 = 6;
pub const SECONDS_PER_DAY: i64 = 86_400;
pub const REWARD_PER_DAY: u64 = 10_000_000; // 10 tokens / day
pub const BURN_BONUS: u64 = 1_000_000_000; // 1000 tokens, one-time
