use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
declare_id!("3WLtmZnhXgctq98BeEKZuXkKCAgMuDNzssrtA6KXqXxW");

const LOCK_TIME_EPOCH: u64 = 100;
const EPOCH_LENGTH: u64 = 10; // seconds
const REWARD_AMOUNT: u64 = 100000;
#[program]
pub mod ogc_reserve {
    use anchor_spl::token::{transfer, Transfer};

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.global_account.mint = ctx.accounts.mint.key();
        Ok(())
    }
    pub fn new_epoch(ctx: Context<NewEpoch>, epoch: u64) -> Result<()> {
        if epoch != ctx.accounts.global_account.epoch + 1 {
            return Err(CustomError::InvalidEpoch.into())
        }
        let mut max_index: usize = 0;
        let mut max: u64 = 0;
        for i in 0..16 {
            if ctx.accounts.prev_epoch_account.reserve_amounts[i] > max {
                max = ctx.accounts.prev_epoch_account.reserve_amounts[i];
                max_index = i;
            }
        }
        ctx.accounts.prev_epoch_account.winner = max_index as u8;
        ctx.accounts.global_account.epoch += 1;
        ctx.accounts.epoch_account.epoch = epoch;
        Ok(())
    }
    pub fn lock(ctx: Context<Lock>, epoch: u64, amount: u64, reserve: u8) -> Result<()> {
        if epoch != ctx.accounts.global_account.epoch {
            return Err(CustomError::InvalidEpoch.into())
        }
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_token_account.to_account_info(),
                    to: ctx.accounts.holder_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            amount,
        )?;
        ctx.accounts.epoch_account.reserve_amounts[reserve as usize] += amount;
        ctx.accounts.lock_account.owner = ctx.accounts.signer.key();
        ctx.accounts.lock_account.amount = amount;
        ctx.accounts.lock_account.reserve = reserve;
        ctx.accounts.lock_account.last_claim_epoch = epoch;
        ctx.accounts.lock_account.unlock_epoch = epoch + LOCK_TIME_EPOCH;
        Ok(())
    }
    pub fn unlock(ctx: Context<Unlock>, amount: u64) -> Result<()> {
        if ctx.accounts.global_account.epoch < ctx.accounts.lock_account.unlock_epoch + 100 {
            return Err(CustomError::NotUnlocked.into())
        }
        if ctx.accounts.global_account.epoch == ctx.accounts.lock_account.last_claim_epoch {
            return Err(CustomError::ClaimedThisEpoch.into())
        }
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.holder_account.to_account_info(),
                    to: ctx.accounts.signer_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            amount
        )?;
        Ok(())
    }
    pub fn claim(ctx: Context<Claim>, prev_epoch: u64, curr_epoch: u64) -> Result<()> {
        if curr_epoch != ctx.accounts.global_account.epoch || prev_epoch >= curr_epoch {
            return Err(CustomError::InvalidEpoch.into())
        }
        if ctx.accounts.lock_account.last_claim_epoch >= prev_epoch {
            // new error message
            return Err(CustomError::InvalidEpoch.into())
        }
        if ctx.accounts.prev_epoch_account.winner != ctx.accounts.lock_account.reserve {
            return Err(CustomError::DidNotWin.into())
        }
        // fix the amount calculation
        let amount = ctx.accounts.lock_account.amount / ctx.accounts.prev_epoch_account.reserve_amounts[ctx.accounts.lock_account.reserve as usize];
        if amount > 0 {
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.program_holder_account.to_account_info(),
                        to: ctx.accounts.signer_token_account.to_account_info(),
                        authority: ctx.accounts.program_authority.to_account_info(),
                    },
                    &[&[b"auth", &[ctx.bumps.program_authority]]]
                ),
                amount
            )?;
        }
        Ok(())
    }
}
#[error_code]
pub enum CustomError {
    #[msg("Invalid Epoch")]
    InvalidEpoch,
    #[msg("Not unlocked")]
    NotUnlocked,
    #[msg("Claimed this epoch")]
    ClaimedThisEpoch,
    #[msg("Did not win")]
    DidNotWin
}

// users lock their ogg for a certain amount of time. They can claim a fractional amount of ogc if their reserve wom
// Each new epoch, they must check in to stake their ogg for the epoch. 
// They can claim at any time
#[account]
pub struct GlobalAccount {
    epoch: u64,
    mint: Pubkey,
}
#[account]
pub struct EpochAccount {
    epoch: u64,
    reserve_amounts: [u64; 16],
    winner: u8,
}
#[account]
pub struct LockAccount {
    owner: Pubkey,
    reserve: u8,
    amount: u64,
    unlock_epoch: u64,
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
        space = 8 + 8 + 32,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    #[account(
        init,
        seeds = [b"epoch", 0_u64.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 1 + 8 * 16
    )]
    pub first_epoch_account: Account<'info, EpochAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"auth"],
        bump,
        payer = signer,
        space = 8,
    )]
    pub program_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"holder"],
        bump,
        payer = signer,
        token::mint = mint,
        token::authority = program_authority,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct NewEpoch<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"epoch", epoch.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 1 + 8 * 16
    )]
    pub epoch_account: Account<'info, EpochAccount>,
    #[account(
        mut,
        seeds = [b"epoch", (epoch - 1).to_le_bytes().as_ref()],
        bump,
    )]
    pub prev_epoch_account: Account<'info, EpochAccount>,
    #[account(
        mut,
        seeds = [b"global"],
        bump,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    pub system_program: Program<'info, System>
}
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct Lock<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"epoch", epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub epoch_account: Account<'info, EpochAccount>,
    #[account(
        init,
        payer = signer,
        space = 8 + 32 + 1 + 8 + 8 + 8
    )]
    pub lock_account: Account<'info, LockAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    #[account(
        constraint = mint.key() == global_account.mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"holder", lock_account.key().as_ref()],
        bump,
        payer = signer,
        token::mint = mint,
        token::authority = program_authority,
    )]
    pub holder_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unlock<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        close = signer,
        constraint = lock_account.owner == signer.key()
    )]
    pub lock_account: Account<'info, LockAccount>,
    #[account(
        mut,
        seeds = [b"holder", lock_account.key().as_ref()],
        bump,
    )]
    pub holder_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(prev_epoch: u64, curr_epoch: u64)]
pub struct Claim<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = lock_account.owner == signer.key()
    )]
    pub lock_account: Account<'info, LockAccount>,
    #[account(
        mut,
        seeds = [b"epoch", prev_epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub prev_epoch_account: Account<'info, EpochAccount>,
    #[account(
        mut,
        seeds = [b"epoch", curr_epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub curr_epoch_account: Account<'info, EpochAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_account: Account<'info, GlobalAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    pub program_authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}