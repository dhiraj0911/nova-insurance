use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::*;
use crate::state::*;

/// Initialize a new insurance pool
pub fn initialize_pool(
    ctx: Context<InitializePool>,
    pool_type: PoolType,
    premium_amount: u64,
    coverage_amount: u64,
    min_validators: u8,
    claim_period: i64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Validate inputs
    require!(premium_amount > 0, NovaError::InvalidPremiumAmount);
    require!(
        coverage_amount > premium_amount,
        NovaError::InvalidCoverageAmount
    );
    require!(min_validators >= 3, NovaError::InsufficientValidators);
    require!(claim_period > 0, NovaError::ClaimPeriodExpired);

    // Get pool key before mutating
    let pool_key = pool.key();
    let authority_key = ctx.accounts.authority.key();
    let vault_key = ctx.accounts.pool_vault.key();

    // Initialize pool state
    pool.pool_id = pool_key;
    pool.pool_type = pool_type;
    pool.authority = authority_key;
    pool.vault = vault_key;
    pool.premium_amount = premium_amount;
    pool.coverage_amount = coverage_amount;
    pool.total_pooled = 0;
    pool.total_members = 0;
    pool.active_claims = 0;
    pool.claim_period = claim_period;
    pool.min_validators = min_validators;
    pool.created_at = clock.unix_timestamp;
    pool.bump = ctx.bumps.pool;

    emit!(PoolCreatedEvent {
        pool_id: pool_key,
        authority: authority_key,
        pool_type,
        premium_amount,
        coverage_amount,
        min_validators,
        claim_period,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Insurance pool created: {} with premium: {} USDC, coverage: {} USDC",
        pool_key,
        premium_amount,
        coverage_amount
    );

    Ok(())
}

/// Join an existing insurance pool
pub fn join_pool(ctx: Context<JoinPool>, coverage_amount: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let user_coverage = &mut ctx.accounts.user_coverage;
    let clock = Clock::get()?;

    // Validate coverage amount
    require!(
        coverage_amount <= pool.coverage_amount,
        NovaError::ExcessiveClaimAmount
    );
    require!(coverage_amount > 0, NovaError::InvalidCoverageAmount);

    // Transfer premium from user to pool vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.pool_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, pool.premium_amount)?;

    // Initialize user coverage
    user_coverage.user = ctx.accounts.user.key();
    user_coverage.pool = pool.key();
    user_coverage.premiums_paid = pool.premium_amount;
    user_coverage.last_payment = clock.unix_timestamp;
    user_coverage.coverage_active = true;
    user_coverage.coverage_amount = coverage_amount;
    user_coverage.claims_made = 0;
    user_coverage.joined_at = clock.unix_timestamp;
    user_coverage.bump = ctx.bumps.user_coverage;

    // Update pool stats
    pool.total_pooled = pool
        .total_pooled
        .checked_add(pool.premium_amount)
        .ok_or(NovaError::InvalidCoverageAmount)?;
    pool.total_members = pool
        .total_members
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;

    emit!(UserJoinedEvent {
        user: ctx.accounts.user.key(),
        pool: pool.key(),
        coverage_amount,
        premium_paid: pool.premium_amount,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "User {} joined pool {} with {} USDC coverage",
        ctx.accounts.user.key(),
        pool.key(),
        coverage_amount
    );

    Ok(())
}

/// Pay monthly premium to maintain coverage
pub fn pay_premium(ctx: Context<PayPremium>) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let user_coverage = &mut ctx.accounts.user_coverage;
    let clock = Clock::get()?;

    // Verify coverage exists
    require!(
        user_coverage.user == ctx.accounts.user.key(),
        NovaError::UnauthorizedValidator
    );

    // Transfer premium from user to pool vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.pool_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, pool.premium_amount)?;

    // Update user coverage
    user_coverage.premiums_paid = user_coverage
        .premiums_paid
        .checked_add(pool.premium_amount)
        .ok_or(NovaError::InvalidPremiumAmount)?;
    user_coverage.last_payment = clock.unix_timestamp;
    user_coverage.coverage_active = true;

    emit!(PremiumPaidEvent {
        user: ctx.accounts.user.key(),
        pool: pool.key(),
        amount: pool.premium_amount,
        total_paid: user_coverage.premiums_paid,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Premium paid: {} USDC by {} for pool {}",
        pool.premium_amount,
        ctx.accounts.user.key(),
        pool.key()
    );

    Ok(())
}

// ============================================================================
// Account Validation Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + InsurancePool::INIT_SPACE,
        seeds = [b"pool", authority.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = pool,
        seeds = [b"vault", pool.key().as_ref()],
        bump
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct JoinPool<'info> {
    #[account(mut)]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        init,
        payer = user,
        space = 8 + UserCoverage::INIT_SPACE,
        seeds = [b"coverage", user.key().as_ref(), pool.key().as_ref()],
        bump
    )]
    pub user_coverage: Account<'info, UserCoverage>,

    #[account(
        mut,
        constraint = pool_vault.key() == pool.vault @ NovaError::UnauthorizedValidator
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ NovaError::UnauthorizedValidator,
        constraint = user_token_account.mint == pool_vault.mint @ NovaError::InvalidPremiumAmount
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct PayPremium<'info> {
    #[account(mut)]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        mut,
        seeds = [b"coverage", user.key().as_ref(), pool.key().as_ref()],
        bump = user_coverage.bump,
        constraint = user_coverage.pool == pool.key() @ NovaError::InactiveCoverage
    )]
    pub user_coverage: Account<'info, UserCoverage>,

    #[account(
        mut,
        constraint = pool_vault.key() == pool.vault @ NovaError::UnauthorizedValidator
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ NovaError::UnauthorizedValidator,
        constraint = user_token_account.mint == pool_vault.mint @ NovaError::InvalidPremiumAmount
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct PoolCreatedEvent {
    pub pool_id: Pubkey,
    pub authority: Pubkey,
    pub pool_type: PoolType,
    pub premium_amount: u64,
    pub coverage_amount: u64,
    pub min_validators: u8,
    pub claim_period: i64,
    pub timestamp: i64,
}

#[event]
pub struct UserJoinedEvent {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub coverage_amount: u64,
    pub premium_paid: u64,
    pub timestamp: i64,
}

#[event]
pub struct PremiumPaidEvent {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub amount: u64,
    pub total_paid: u64,
    pub timestamp: i64,
}
