use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::errors::NovaError;
use crate::state::*;

/// Instruction: Deposit idle pool funds to Kamino yield vault
/// 
/// This instruction moves a specified amount of idle USDC from the insurance pool
/// vault to a yield-generating protocol (Kamino) to earn returns on unused funds.
/// 
/// Security considerations:
/// - Only pool authority can call this
/// - Cannot deposit more than available idle funds
/// - Must maintain minimum reserve for immediate claim payouts
#[derive(Accounts)]
pub struct DepositToYield<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool.pool_id.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        mut,
        constraint = vault.key() == pool.vault @ NovaError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Kamino yield vault token account (placeholder for MVP)
    #[account(mut)]
    pub yield_vault: Account<'info, TokenAccount>,

    #[account(
        constraint = authority.key() == pool.authority @ NovaError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// Instruction: Withdraw funds from Kamino yield vault back to pool
/// 
/// This instruction pulls funds back from the yield protocol to the insurance pool
/// vault, typically to prepare for claim payouts or increase liquidity.
#[derive(Accounts)]
pub struct WithdrawFromYield<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool.pool_id.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        mut,
        constraint = vault.key() == pool.vault @ NovaError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Kamino yield vault token account (placeholder for MVP)
    #[account(mut)]
    pub yield_vault: Account<'info, TokenAccount>,

    /// Kamino vault authority (placeholder)
    /// CHECK: This is a placeholder for Kamino's vault authority
    pub yield_vault_authority: AccountInfo<'info>,

    #[account(
        constraint = authority.key() == pool.authority @ NovaError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// Handler: Deposit idle funds to yield vault
/// 
/// Algorithm:
/// 1. Calculate idle funds = total_pooled - (active_claims * avg_claim_amount)
/// 2. Ensure minimum reserve (20% of total_pooled) remains in vault
/// 3. Transfer excess funds to Kamino yield vault
/// 4. Update pool's yield_deposited amount
/// 5. Record timestamp for yield tracking
/// 
/// Params:
/// - amount: Amount of USDC to deposit to yield vault
pub fn deposit_to_yield(ctx: Context<DepositToYield>, amount: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let vault = &ctx.accounts.vault;
    let clock = Clock::get()?;

    // Validate amount
    require!(amount > 0, NovaError::InvalidCoverageAmount);

    // Calculate minimum reserve (20% of total pooled)
    let min_reserve = pool.total_pooled
        .checked_mul(20)
        .ok_or(NovaError::MathOverflow)?
        .checked_div(100)
        .ok_or(NovaError::MathOverflow)?;

    // Ensure we maintain minimum reserve
    let available_for_yield = vault.amount
        .checked_sub(min_reserve)
        .ok_or(NovaError::InsufficientPoolFunds)?;

    require!(
        amount <= available_for_yield,
        NovaError::InsufficientPoolFunds
    );

    // Transfer to yield vault using PDA authority
    let pool_id = pool.pool_id.key();
    let bump = pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"pool",
        pool_id.as_ref(),
        &[bump],
    ]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.yield_vault.to_account_info(),
        authority: pool.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    token::transfer(cpi_ctx, amount)?;

    // Update pool yield tracking
    pool.yield_deposited = pool.yield_deposited
        .checked_add(amount)
        .ok_or(NovaError::MathOverflow)?;

    pool.last_yield_update = clock.unix_timestamp;

    emit!(YieldDepositedEvent {
        pool: pool.key(),
        amount,
        total_yield_deposited: pool.yield_deposited,
        timestamp: clock.unix_timestamp,
    });

    msg!("Deposited {} USDC to yield vault", amount);

    Ok(())
}

/// Handler: Withdraw funds from yield vault back to pool
/// 
/// Algorithm:
/// 1. Validate withdrawal amount doesn't exceed deposited amount
/// 2. Calculate accrued yield (current vault balance - deposited amount)
/// 3. Transfer funds from Kamino vault back to pool vault
/// 4. Update pool's yield_deposited and yield_earned
/// 5. Record timestamp for accounting
/// 
/// Params:
/// - amount: Amount of USDC to withdraw from yield vault
pub fn withdraw_from_yield(ctx: Context<WithdrawFromYield>, amount: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Validate amount
    require!(amount > 0, NovaError::InvalidCoverageAmount);
    require!(
        amount <= pool.yield_deposited,
        NovaError::InsufficientPoolFunds
    );

    // Calculate yield earned before withdrawal
    let yield_vault_balance = ctx.accounts.yield_vault.amount;
    let principal = pool.yield_deposited;

    if yield_vault_balance > principal {
        let earned_yield = yield_vault_balance
            .checked_sub(principal)
            .ok_or(NovaError::MathOverflow)?;

        pool.yield_earned = pool.yield_earned
            .checked_add(earned_yield)
            .ok_or(NovaError::MathOverflow)?;

        msg!("Earned yield: {} USDC", earned_yield);
    }

    // For MVP, we're using a placeholder transfer
    // In production, this would interact with Kamino's withdraw instruction
    // 
    // Example Kamino integration (commented for MVP):
    // let kamino_withdraw_accounts = kamino::cpi::accounts::Withdraw {
    //     vault: ctx.accounts.yield_vault.to_account_info(),
    //     user_token_account: ctx.accounts.vault.to_account_info(),
    //     vault_authority: ctx.accounts.yield_vault_authority.to_account_info(),
    //     token_program: ctx.accounts.token_program.to_account_info(),
    // };
    // 
    // kamino::cpi::withdraw(
    //     CpiContext::new(kamino_program, kamino_withdraw_accounts),
    //     amount,
    // )?;

    // MVP: Simulate withdrawal (in production, actual CPI call to Kamino)
    // Transfer from yield vault to pool vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.yield_vault.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.yield_vault_authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );

    token::transfer(cpi_ctx, amount)?;

    // Update pool yield tracking
    pool.yield_deposited = pool.yield_deposited
        .checked_sub(amount)
        .ok_or(NovaError::MathOverflow)?;

    pool.total_pooled = pool.total_pooled
        .checked_add(amount)
        .ok_or(NovaError::MathOverflow)?;

    pool.last_yield_update = clock.unix_timestamp;

    emit!(YieldWithdrawnEvent {
        pool: pool.key(),
        amount,
        yield_earned: pool.yield_earned,
        total_yield_deposited: pool.yield_deposited,
        timestamp: clock.unix_timestamp,
    });

    msg!("Withdrew {} USDC from yield vault", amount);

    Ok(())
}

/// Helper function to calculate idle funds available for yield
/// 
/// Idle funds = Total pooled - Reserved for active claims - Minimum reserve
/// 
/// This helps determine how much can safely be deposited to yield protocols
pub fn calculate_idle_funds(pool: &InsurancePool, vault_balance: u64) -> Result<u64> {
    // Estimate reserved funds for active claims
    // Assuming average claim is 50% of coverage amount
    let avg_claim_estimate = pool.coverage_amount
        .checked_div(2)
        .ok_or(NovaError::MathOverflow)?;

    let reserved_for_claims = (pool.active_claims as u64)
        .checked_mul(avg_claim_estimate)
        .ok_or(NovaError::MathOverflow)?;

    // Calculate 20% minimum reserve
    let min_reserve = pool.total_pooled
        .checked_mul(20)
        .ok_or(NovaError::MathOverflow)?
        .checked_div(100)
        .ok_or(NovaError::MathOverflow)?;

    // Calculate idle funds
    let total_reserved = reserved_for_claims
        .checked_add(min_reserve)
        .ok_or(NovaError::MathOverflow)?;

    if vault_balance > total_reserved {
        Ok(vault_balance
            .checked_sub(total_reserved)
            .ok_or(NovaError::MathOverflow)?)
    } else {
        Ok(0)
    }
}

/// Helper function to calculate APY from yield earned
/// 
/// Simple APY calculation for tracking yield performance
pub fn calculate_apy(
    yield_earned: u64,
    principal: u64,
    time_elapsed_seconds: i64,
) -> Result<u64> {
    if principal == 0 || time_elapsed_seconds <= 0 {
        return Ok(0);
    }

    // APY = (yield_earned / principal) * (seconds_per_year / time_elapsed) * 100
    let seconds_per_year: u64 = 365 * 24 * 60 * 60;

    let yield_ratio = (yield_earned as u128)
        .checked_mul(10000) // Scale for precision
        .ok_or(NovaError::MathOverflow)?
        .checked_div(principal as u128)
        .ok_or(NovaError::MathOverflow)?;

    let annualized = yield_ratio
        .checked_mul(seconds_per_year as u128)
        .ok_or(NovaError::MathOverflow)?
        .checked_div(time_elapsed_seconds as u128)
        .ok_or(NovaError::MathOverflow)?;

    // Divide by 100 to get percentage (keeping 2 decimal precision)
    let apy = (annualized as u64)
        .checked_div(100)
        .ok_or(NovaError::MathOverflow)?;

    Ok(apy)
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct YieldDepositedEvent {
    pub pool: Pubkey,
    pub amount: u64,
    pub total_yield_deposited: u64,
    pub timestamp: i64,
}

#[event]
pub struct YieldWithdrawnEvent {
    pub pool: Pubkey,
    pub amount: u64,
    pub yield_earned: u64,
    pub total_yield_deposited: u64,
    pub timestamp: i64,
}
