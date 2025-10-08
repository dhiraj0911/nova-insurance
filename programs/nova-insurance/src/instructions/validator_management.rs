use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer as SystemTransfer};

use crate::errors::*;
use crate::state::*;

/// Stake SOL to become a validator
pub fn stake_as_validator(
    ctx: Context<StakeAsValidator>,
    stake_amount: u64,
) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    // Validate minimum stake requirement (0.1 SOL minimum)
    const MIN_STAKE: u64 = 100_000_000; // 0.1 SOL in lamports
    require!(
        stake_amount >= MIN_STAKE,
        NovaError::InsufficientValidators
    );

    // Get keys before mutation
    let validator_key = ctx.accounts.validator.key();

    // Transfer SOL from validator to validator stake account
    let transfer_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        SystemTransfer {
            from: ctx.accounts.validator.to_account_info(),
            to: ctx.accounts.validator_stake.to_account_info(),
        },
    );
    transfer(transfer_ctx, stake_amount)?;

    // Now initialize validator stake after transfer
    let validator_stake = &mut ctx.accounts.validator_stake;
    validator_stake.validator = validator_key;
    validator_stake.stake_amount = stake_amount;
    validator_stake.validations_completed = 0;
    validator_stake.successful_validations = 0;
    validator_stake.reputation_score = ValidatorStake::INITIAL_REPUTATION;
    validator_stake.last_validation = 0;
    validator_stake.bump = ctx.bumps.validator_stake;

    // Register validator in pool's validator registry
    let validator_registry = &mut ctx.accounts.validator_registry;
    
    // Add validator to registry if not already present
    if !validator_registry.validators.contains(&validator_key) {
        require!(
            validator_registry.validators.len() < validator_registry.validators.capacity(),
            NovaError::InsufficientValidators
        );
        validator_registry.validators.push(validator_key);
        validator_registry.total_validators = validator_registry
            .total_validators
            .checked_add(1)
            .ok_or(NovaError::InvalidCoverageAmount)?;
    }

    emit!(ValidatorStakedEvent {
        validator: validator_key,
        pool: pool.key(),
        stake_amount,
        reputation_score: ValidatorStake::INITIAL_REPUTATION,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Validator {} staked {} lamports for pool {}",
        validator_key,
        stake_amount,
        pool.key()
    );

    Ok(())
}

/// Validate a claim (approve or reject)
pub fn validate_claim(
    ctx: Context<ValidateClaim>,
    approve: bool,
    reason: String,
) -> Result<()> {
    let claim = &mut ctx.accounts.claim_request;
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    // Verify claim is in validation status
    require!(
        claim.status == ClaimStatus::UnderValidation || claim.status == ClaimStatus::Pending,
        NovaError::ClaimPeriodExpired
    );

    // Verify validator is assigned to this claim
    let validator_key = ctx.accounts.validator.key();
    require!(
        claim.validators_assigned.contains(&validator_key),
        NovaError::UnauthorizedValidator
    );

    // Check if validator already validated
    let already_validated = claim.validations
        .iter()
        .any(|v| v.validator == validator_key);
    require!(!already_validated, NovaError::DuplicateValidation);

    // Validate reason length
    require!(
        reason.len() <= 200,
        NovaError::InvalidCoverageAmount
    );

    // Record validation
    claim.validations.push(Validation {
        validator: validator_key,
        approved: approve,
        reason: reason.clone(),
        timestamp: clock.unix_timestamp,
    });

    // Update counts
    if approve {
        claim.approvals = claim.approvals.checked_add(1).ok_or(NovaError::InvalidCoverageAmount)?;
    } else {
        claim.rejections = claim.rejections.checked_add(1).ok_or(NovaError::InvalidCoverageAmount)?;
    }

    // Check if all validators have responded
    let total_validations = claim.approvals.checked_add(claim.rejections).ok_or(NovaError::InvalidCoverageAmount)?;
    let required_validations = claim.validators_assigned.len() as u8;

    // Determine if claim is finalized
    let is_finalized = total_validations >= required_validations;
    let majority_threshold = (required_validations / 2) + 1;
    let is_approved = claim.approvals >= majority_threshold;

    if is_finalized {
        if is_approved {
            claim.status = ClaimStatus::Approved;
            claim.resolved_at = Some(clock.unix_timestamp);
            claim.payout_amount = Some(claim.amount_requested);
            msg!("Claim {} APPROVED", claim.claim_id);
        } else {
            claim.status = ClaimStatus::Rejected;
            claim.resolved_at = Some(clock.unix_timestamp);
            msg!("Claim {} REJECTED", claim.claim_id);
        }

        // Update validator reputation based on whether they voted with majority
        let voted_with_majority = (is_approved && approve) || (!is_approved && !approve);
        update_validator_reputation(
            &mut ctx.accounts.validator_stake,
            voted_with_majority,
            pool,
        )?;
    } else {
        // Still waiting for more validations
        claim.status = ClaimStatus::UnderValidation;
        
        // Update validator stats
        ctx.accounts.validator_stake.validations_completed = ctx.accounts.validator_stake
            .validations_completed
            .checked_add(1)
            .ok_or(NovaError::InvalidCoverageAmount)?;
        ctx.accounts.validator_stake.last_validation = clock.unix_timestamp;
    }

    emit!(ClaimValidatedEvent {
        claim_id: claim.claim_id,
        validator: validator_key,
        approved: approve,
        claim_status: claim.status,
        approvals: claim.approvals,
        rejections: claim.rejections,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Validator {} {} claim {} - Status: {:?}",
        validator_key,
        if approve { "APPROVED" } else { "REJECTED" },
        claim.claim_id,
        claim.status
    );

    Ok(())
}

/// Update validator reputation and stats based on voting outcome
fn update_validator_reputation(
    validator_stake: &mut ValidatorStake,
    voted_with_majority: bool,
    pool: &InsurancePool,
) -> Result<()> {
    // Update validation count
    validator_stake.validations_completed = validator_stake
        .validations_completed
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;
    validator_stake.last_validation = Clock::get()?.unix_timestamp;

    if voted_with_majority {
        // Reward for correct vote
        validator_stake.successful_validations = validator_stake
            .successful_validations
            .checked_add(1)
            .ok_or(NovaError::InvalidCoverageAmount)?;
        
        // Increase reputation
        validator_stake.reputation_score = validator_stake
            .reputation_score
            .saturating_add(100)
            .min(ValidatorStake::MAX_REPUTATION);

        msg!("Validator {} rewarded: +100 reputation", validator_stake.validator);
    } else {
        // Slash for incorrect vote
        slash_validator(validator_stake, pool)?;
    }

    Ok(())
}

/// Slash validator for dishonest behavior
fn slash_validator(validator_stake: &mut ValidatorStake, pool: &InsurancePool) -> Result<()> {
    // Calculate slash amount based on pool's minimum validators requirement
    // Higher requirement = more severe slashing
    let slash_percentage = pool.min_validators as u32 * 2; // 2% per min validator
    let slash_amount = (validator_stake.stake_amount as u128)
        .checked_mul(slash_percentage as u128)
        .ok_or(NovaError::InvalidCoverageAmount)?
        .checked_div(100)
        .ok_or(NovaError::InvalidCoverageAmount)? as u64;

    // Deduct from reputation
    validator_stake.reputation_score = validator_stake
        .reputation_score
        .saturating_sub(200); // -200 reputation for incorrect vote

    // Record slashed amount (actual SOL slashing would be in separate instruction)
    validator_stake.stake_amount = validator_stake
        .stake_amount
        .saturating_sub(slash_amount);

    msg!(
        "Validator {} slashed {} lamports and -200 reputation",
        validator_stake.validator,
        slash_amount
    );

    Ok(())
}

// ============================================================================
// Account Validation Contexts
// ============================================================================

#[derive(Accounts)]
pub struct StakeAsValidator<'info> {
    #[account(
        init,
        payer = validator,
        space = 8 + ValidatorStake::INIT_SPACE,
        seeds = [b"validator", validator.key().as_ref(), pool.key().as_ref()],
        bump
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    #[account(
        mut,
        seeds = [b"validator_registry", pool.key().as_ref()],
        bump = validator_registry.bump
    )]
    pub validator_registry: Account<'info, ValidatorRegistry>,

    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub validator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ValidateClaim<'info> {
    #[account(mut)]
    pub claim_request: Account<'info, ClaimRequest>,

    #[account(
        mut,
        seeds = [b"validator", validator.key().as_ref(), pool.key().as_ref()],
        bump = validator_stake.bump,
        constraint = validator_stake.validator == validator.key() @ NovaError::UnauthorizedValidator
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    pub pool: Account<'info, InsurancePool>,

    pub validator: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct ValidatorStakedEvent {
    pub validator: Pubkey,
    pub pool: Pubkey,
    pub stake_amount: u64,
    pub reputation_score: u32,
    pub timestamp: i64,
}

#[event]
pub struct ClaimValidatedEvent {
    pub claim_id: Pubkey,
    pub validator: Pubkey,
    pub approved: bool,
    pub claim_status: ClaimStatus,
    pub approvals: u8,
    pub rejections: u8,
    pub timestamp: i64,
}
