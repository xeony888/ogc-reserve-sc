use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, transfer, Transfer};
declare_id!("3WLtmZnhXgctq98BeEKZuXkKCAgMuDNzssrtA6KXqXxW");

const ADMIN: &str = "Ddi1GaugnX9yQz1WwK1b12m4o23rK1krZQMcnt2aNW97";
const EPOCH_FIRST_END_TIME: u64 = 0; // use this to set first end time, making epochs end at a specific time of the day
#[program]
pub mod ogc_reserve {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.global_data_account.epoch_lock_time = 1;
        ctx.accounts.global_data_account.epoch_end_time = EPOCH_FIRST_END_TIME;
        ctx.accounts.global_data_account.epoch_length = 10;
        ctx.accounts.global_data_account.reward_percent = 5;
        ctx.accounts.global_data_account.mint = ctx.accounts.mint.key();
        Ok(())
    }
    pub fn deposit_ogg(ctx: Context<DepositOgg>, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_token_account.to_account_info(),
                    to: ctx.accounts.program_holder_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            amount
        )?;
        Ok(())
    }
    pub fn withdraw_ogg(ctx: Context<WithdrawOgg>, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.program_holder_account.to_account_info(),
                    to: ctx.accounts.signer_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            amount,
        )?;
        Ok(())
    }
    pub fn new_epoch(ctx: Context<NewEpoch>, epoch: u64) -> Result<()> {
        let time = Clock::get()?.unix_timestamp as u64;
        if time < ctx.accounts.global_data_account.epoch_end_time {
            return Err(CustomError::EpochNotOver.into())
        }
        ctx.accounts.global_data_account.epoch += 1;
        ctx.accounts.global_data_account.epoch_end_time += ctx.accounts.global_data_account.epoch_length;
        let mut max: u64 = 0;
        let mut max_index: usize = 0;
        for i in 0..16 {
            if ctx.accounts.prev_epoch_account.fields[i] > max {
                max = ctx.accounts.prev_epoch_account.fields[i];
                max_index = i;
            }
        }
        ctx.accounts.prev_epoch_account.winner = max_index as u64;
        ctx.accounts.prev_epoch_account.reward = ctx.accounts.program_holder_account.amount * ctx.accounts.global_data_account.reward_percent / 100;
        Ok(())
    }
    pub fn create_data_account(ctx: Context<CreateDataAccount>) -> Result<()> {
        Ok(())
    }
    pub fn create_lock_account(ctx: Context<CreateLockAccount>, epoch: u64) -> Result<()> {
        Ok(())
    }
    pub fn lock(ctx: Context<Lock>, epoch: u64, amount: u64) -> Result<()> {
        // no point in this check
        // let time = Clock::get()?.unix_timestamp as u64;
        // if ctx.accounts.global_data_account.epoch_end_time > time {
        //     return Err(CustomError::EpochExpired.into())
        // }
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_token_account.to_account_info(),
                    to: ctx.accounts.signer_holder_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            amount
        )?;
        ctx.accounts.lock_account.unlock_epoch = ctx.accounts.global_data_account.epoch + ctx.accounts.global_data_account.epoch_lock_time;
        ctx.accounts.lock_account.amount += amount;
        ctx.accounts.user_data_account.amount += amount;
        Ok(())
    }
    pub fn unlock(ctx: Context<Unlock>, epoch: u64, amount: u64) -> Result<()> {
        if amount > ctx.accounts.lock_account.amount {
            return Err(CustomError::ExceedsBalanceOfLockAccount.into())
        }
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_holder_account.to_account_info(),
                    to: ctx.accounts.signer_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.lock_account]]]
            ),
            amount
        )?;
        ctx.accounts.lock_account.amount -= amount;
        ctx.accounts.user_data_account.amount -= amount;
        if ctx.accounts.lock_account.amount == 0 {
            ctx.accounts.lock_account.close(ctx.accounts.signer.to_account_info())?;
        }
        Ok(())
    }
    pub fn vote(ctx: Context<Vote>, epoch: u64, amounts: [u64; 16]) -> Result<()> {
        if epoch != ctx.accounts.global_data_account.epoch {
            return Err(CustomError::IncorrectEpochNum.into())
        }
        let mut sum = 0;
        for i in 0..16 {
            ctx.accounts.epoch_account.fields[i] += amounts[i];
            ctx.accounts.vote_account.fields[i] = amounts[i];
            sum += amounts[i];
        }
        if sum > ctx.accounts.user_data_account.amount {
            return Err(CustomError::NotEnoughStaked.into())
        }
        Ok(())
    }
    pub fn claim(ctx: Context<Claim>, epoch: u64) -> Result<()> {
        if epoch >= ctx.accounts.global_data_account.epoch {
            return Err(CustomError::IncorrectEpochNum.into())
        }
        let reward = ctx.accounts.vote_account.fields[ctx.accounts.epoch_account.winner as usize] * ctx.accounts.epoch_account.reward / ctx.accounts.epoch_account.fields[ctx.accounts.epoch_account.winner as usize];
        if reward > 0 {
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.program_holder_account.to_account_info(),
                        to: ctx.accounts.signer_token_account.to_account_info(),
                        authority: ctx.accounts.program_authority.to_account_info()
                    },
                    &[&[b"auth", &[ctx.bumps.program_authority]]]
                ),
                reward,
            )?;
        }
        Ok(())
    }
}

#[error_code]
pub enum CustomError {
    #[msg("Invalid mint account")]
    InvalidMintAccount,
    #[msg("Incorrect epoch num")]
    IncorrectEpochNum,
    #[msg("Epoch Expired")]
    EpochExpired,
    #[msg("Account not unlocked")]
    AccountNotUnlocked,
    #[msg("Exceeds balance of lock account")]
    ExceedsBalanceOfLockAccount,
    #[msg("Epoch not over")]
    EpochNotOver,
    #[msg("Not enough staked")]
    NotEnoughStaked,
    #[msg("Invalid signer")]
    InvalidSigner
}

#[account]
pub struct GlobalDataAccount {
    pub epoch: u64,
    pub epoch_end_time: u64,
    pub epoch_lock_time: u64,
    pub epoch_length: u64,
    pub reward_percent: u64,
    pub mint: Pubkey,
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"global"],
        bump,
        space = 8 + 8 + 8 + 8 + 8 + 8 + 32,
        payer = signer,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        init,
        seeds = [b"holder"],
        bump,
        token::mint = mint,
        token::authority = program_authority,
        payer = signer,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [b"auth"],
        bump,
        space = 8,
        payer = signer,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"epoch", 0_u64.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 8 + 8 * 16
    )]
    pub first_epoch_account: Account<'info, EpochAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct DepositOgg<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct WithdrawOgg<'info> {
    #[account(
        constraint = signer.key() == ADMIN.parse::<Pubkey>().unwrap() @ CustomError::InvalidSigner
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
#[account]
pub struct EpochAccount {
    pub fields: [u64; 16],
    pub winner: u64,
    pub reward: u64,
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
        space = 8 + 8 + 8 + 8 * 16,
        constraint = global_data_account.epoch + 1 == epoch @ CustomError::IncorrectEpochNum
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
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct LockAccount {
    pub unlock_epoch: u64,
    pub amount: u64,
}
#[account]
pub struct UserDataAccount {
    pub amount: u64,
}
#[derive(Accounts)]
pub struct CreateDataAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"data", signer.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8,
    )]
    pub user_data_account: Account<'info, UserDataAccount>,
    #[account(
        constraint = mint.key() == global_data_account.mint @ CustomError::InvalidMintAccount
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"holder", signer.key().as_ref()],
        bump,
        payer = signer,
        token::mint = mint,
        token::authority = program_authority,
    )]
    pub signer_holder_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct CreateLockAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"lock", signer.key().as_ref(), epoch.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 8,
    )]
    pub lock_account: Account<'info, LockAccount>,
    pub system_program: Program<'info, System>,
}
// maybe add this, maybe not
// #[account]
// pub struct UserStatsAccount {
//     pub epochs: u64,
//     pub claimed: u64,
// }
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct Lock<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"holder", signer.key().as_ref()],
        bump,
    )]
    pub signer_holder_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        seeds = [b"lock", signer.key().as_ref(), epoch.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 8
    )]
    pub lock_account: Account<'info, LockAccount>,
    #[account(
        mut,
        seeds = [b"data", signer.key().as_ref()],
        bump,
    )]
    pub user_data_account: Account<'info, UserDataAccount>,
    #[account(
        seeds = [b"global"],
        bump,
        constraint = epoch == global_data_account.epoch @ CustomError::IncorrectEpochNum
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct Unlock<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"holder", signer.key().as_ref()],
        bump,
    )]
    pub signer_holder_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"lock", signer.key().as_ref(), epoch.to_le_bytes().as_ref()],
        bump,
        constraint = lock_account.unlock_epoch <= global_data_account.epoch @ CustomError::AccountNotUnlocked
    )]
    pub lock_account: Account<'info, LockAccount>,
    #[account(
        mut,
        seeds = [b"data", signer.key().as_ref()],
        bump,
    )]
    pub user_data_account: Account<'info, UserDataAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct VoteAccount {
    fields: [u64; 16]
}
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct Vote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"vote", signer.key().as_ref(), epoch.to_le_bytes().as_ref()], 
        bump,
        payer = signer,
        space = 8 + 16 * 8,
    )]
    pub vote_account: Account<'info, VoteAccount>,
    #[account(
        mut,
        seeds = [b"epoch", epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub epoch_account: Account<'info, EpochAccount>,
    #[account(
        seeds = [b"data", signer.key().as_ref()],
        bump,
    )]
    pub user_data_account: Account<'info, UserDataAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct Claim<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vote", signer.key().as_ref(), epoch.to_le_bytes().as_ref()],
        bump,
        close = signer
    )]
    pub vote_account: Account<'info, VoteAccount>,
    #[account(
        seeds = [b"epoch", epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub epoch_account: Account<'info, EpochAccount>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_data_account: Account<'info, GlobalDataAccount>,
    #[account(
        mut,
        seeds = [b"holder"],
        bump,
    )]
    pub program_holder_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}