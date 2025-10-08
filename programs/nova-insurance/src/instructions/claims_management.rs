use anchor_lang::prelude::*;

use crate::errors::*;
use crate::state::*;

/// Submit a new insurance claim
pub fn submit_claim(
    ctx: Context<SubmitClaim>,
    incident_type: IncidentType,
    amount_requested: u64,
    incident_timestamp: i64,
    description: String,
) -> Result<()> {
    let claim = &mut ctx.accounts.claim_request;
    let user_coverage = &ctx.accounts.user_coverage;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Verify user has active coverage
    require!(
        user_coverage.coverage_active,
        NovaError::InactiveCoverage
    );

    // Verify the coverage belongs to this user and pool
    require!(
        user_coverage.user == ctx.accounts.claimant.key(),
        NovaError::UnauthorizedValidator
    );
    require!(
        user_coverage.pool == pool.key(),
        NovaError::InactiveCoverage
    );

    // Validate claim amount against user's coverage
    require!(
        amount_requested > 0,
        NovaError::InvalidCoverageAmount
    );
    require!(
        amount_requested <= user_coverage.coverage_amount,
        NovaError::ExcessiveClaimAmount
    );

    // Validate claim period - claim must be for recent incident
    let time_since_incident = clock.unix_timestamp.saturating_sub(incident_timestamp);
    require!(
        time_since_incident >= 0,
        NovaError::ClaimPeriodExpired
    );
    require!(
        time_since_incident <= pool.claim_period,
        NovaError::ClaimPeriodExpired
    );

    // Validate user joined before incident (prevent fraud)
    require!(
        user_coverage.joined_at <= incident_timestamp,
        NovaError::ClaimPeriodExpired
    );

    // Validate description length
    require!(
        description.len() <= 100,
        NovaError::InvalidCoverageAmount
    );

    // Get keys before mutation
    let claim_key = claim.key();
    let claimant_key = ctx.accounts.claimant.key();
    let pool_key = pool.key();

    // Initialize claim request
    claim.claim_id = claim_key;
    claim.claimant = claimant_key;
    claim.pool = pool_key;
    claim.amount_requested = amount_requested;
    claim.incident_type = incident_type;
    claim.incident_timestamp = incident_timestamp;
    claim.description = description.clone();
    claim.validators_assigned = Vec::new();
    claim.validations = Vec::new();
    claim.approvals = 0;
    claim.rejections = 0;
    claim.status = ClaimStatus::Pending;
    claim.vrf_result = None;
    claim.created_at = clock.unix_timestamp;
    claim.resolved_at = None;
    claim.payout_amount = None;
    claim.bump = ctx.bumps.claim_request;

    // Update pool active claims counter
    pool.active_claims = pool
        .active_claims
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;

    emit!(ClaimSubmittedEvent {
        claim_id: claim_key,
        claimant: claimant_key,
        pool: pool_key,
        amount_requested,
        incident_type,
        incident_timestamp,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Claim {} submitted by {} for {} USDC - Type: {:?}",
        claim_key,
        claimant_key,
        amount_requested,
        incident_type
    );

    Ok(())
}

// ============================================================================
// Account Validation Contexts
// ============================================================================

#[derive(Accounts)]
pub struct SubmitClaim<'info> {
    #[account(
        init,
        payer = claimant,
        space = 8 + ClaimRequest::INIT_SPACE,
        seeds = [
            b"claim",
            claimant.key().as_ref(),
            pool.key().as_ref(),
            &clock.unix_timestamp.to_le_bytes()
        ],
        bump
    )]
    pub claim_request: Account<'info, ClaimRequest>,

    #[account(mut)]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        seeds = [b"coverage", claimant.key().as_ref(), pool.key().as_ref()],
        bump = user_coverage.bump,
        constraint = user_coverage.pool == pool.key() @ NovaError::InactiveCoverage,
        constraint = user_coverage.user == claimant.key() @ NovaError::UnauthorizedValidator
    )]
    pub user_coverage: Account<'info, UserCoverage>,

    #[account(mut)]
    pub claimant: Signer<'info>,

    pub system_program: Program<'info, System>,
    
    /// Clock sysvar for timestamp
    pub clock: Sysvar<'info, Clock>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct ClaimSubmittedEvent {
    pub claim_id: Pubkey,
    pub claimant: Pubkey,
    pub pool: Pubkey,
    pub amount_requested: u64,
    pub incident_type: IncidentType,
    pub incident_timestamp: i64,
    pub timestamp: i64,
}
