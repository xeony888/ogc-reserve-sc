use anchor_lang::prelude::*;
declare_id!("3WLtmZnhXgctq98BeEKZuXkKCAgMuDNzssrtA6KXqXxW");

const LOCK_TIME_EPOCH: u64 = 100;
const EPOCH_LENGTH: u64 = 10; // seconds
#[program]
pub mod ogc_reserve {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
    pub fn new_epoch(ctx: Context<NewEpoch>, epoch: u64) -> Result<()> {
        Ok(())
    }
    pub fn lock(ctx: Context<Lock>) -> Result<()> {
        Ok(())
    }
    pub fn unlock(ctx: Context<Unlock>) -> Result<()> {
        Ok(())
    }
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        Ok(())
    }
}


// users lock their ogg for a certain amount of time. They can claim a fractional amount of ogc if their reserve wom
// Each new epoch, they must check in to stake their ogg for the epoch. 
// They can claim at any time
#[account]
pub struct GlobalAccount {
    epoch: u64,

}
#[account]
pub struct EpochAccount {
    epoch: u64,
    reserve_amounts: [u64; 16],
}
#[account]
pub struct LockAccount {
    owner: Pubkey,
    reserve: u8,
    amount: u64,
    unlock_time: u64,
    last_claim_epoch: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"global"],
        bump,
        payer = signer,
        space = 8 + 8,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    pub program_authority: AccountInfo<'info>,
    pub program_holder_account: Account<'info, TokenAccount>,
}