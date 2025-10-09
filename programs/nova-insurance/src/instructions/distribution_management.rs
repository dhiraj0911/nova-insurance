use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::NovaError;
use crate::state::*;

/// Fulfill VRF callback for validator selection
/// Uses randomness to select N validators from active pool and assign to claims
pub fn fulfill_validator_selection(
    ctx: Context<FulfillValidatorSelection>,
    randomness: [u8; 32],
) -> Result<()> {
    let vrf_state = &mut ctx.accounts.vrf_state;
    let claim = &mut ctx.accounts.claim_request;
    let validator_registry = &ctx.accounts.validator_registry;
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    // Verify VRF state belongs to pool
    require!(
        vrf_state.pool == pool.key(),
        NovaError::InvalidPoolType
    );

    // Store randomness result
    vrf_state.last_randomness = Some(randomness);
    vrf_state.last_timestamp = clock.unix_timestamp;
    vrf_state.requests_completed = vrf_state
        .requests_completed
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;

    // Get number of validators to select (min_validators from pool)
    let num_validators = pool.min_validators as usize;
    let available_validators = &validator_registry.validators;

    // Ensure we have enough validators
    require!(
        available_validators.len() >= num_validators,
        NovaError::InsufficientValidators
    );

    // Use randomness to select validators
    let mut selected_validators = Vec::new();
    let mut used_indices = Vec::new();

    // Convert randomness to selection indices
    for i in 0..num_validators {
        // Use different bytes of randomness for each selection
        let random_bytes = [
            randomness[i * 4],
            randomness[i * 4 + 1],
            randomness[i * 4 + 2],
            randomness[i * 4 + 3],
        ];
        let random_value = u32::from_le_bytes(random_bytes);
        
        // Find an unused validator index
        let mut attempts = 0;
        loop {
            let index = ((random_value as usize + attempts) % available_validators.len()) as usize;
            
            if !used_indices.contains(&index) {
                used_indices.push(index);
                selected_validators.push(available_validators[index]);
                break;
            }
            
            attempts += 1;
            if attempts >= available_validators.len() {
                return Err(NovaError::InsufficientValidators.into());
            }
        }
    }

    // Assign validators to claim
    claim.validators_assigned = selected_validators;
    claim.vrf_result = Some(randomness);
    claim.status = ClaimStatus::UnderValidation;

    // Remove claim from pending queue
    if let Some(pos) = vrf_state.pending_claims.iter().position(|&c| c == claim.key()) {
        vrf_state.pending_claims.remove(pos);
    }

    emit!(ValidatorSelectionFulfilledEvent {
        claim_id: claim.key(),
        validators: claim.validators_assigned.clone(),
        randomness,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Validators selected for claim {}: {:?}",
        claim.key(),
        claim.validators_assigned
    );

    Ok(())
}

/// Initialize distribution queue for a pool
pub fn initialize_distribution_queue(
    ctx: Context<InitializeDistributionQueue>,
) -> Result<()> {
    let queue = &mut ctx.accounts.distribution_queue;
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    queue.pool = pool.key();
    queue.total_approved_claims = 0;
    queue.total_requested_amount = 0;
    queue.available_funds = pool.total_pooled;
    queue.pending_claims = Vec::new();
    queue.selected_claims = Vec::new();
    queue.vrf_result = None;
    queue.is_oversubscribed = false;
    queue.distribution_round = 0;
    queue.last_distribution = clock.unix_timestamp;
    queue.bump = *ctx.bumps.get("distribution_queue").unwrap();

    emit!(DistributionQueueInitializedEvent {
        pool: pool.key(),
        timestamp: clock.unix_timestamp,
    });

    msg!("Distribution queue initialized for pool {}", pool.key());

    Ok(())
}

/// Distribute claims - handles both normal and oversubscribed scenarios
pub fn distribute_claims(
    ctx: Context<DistributeClaims>,
    randomness: Option<[u8; 32]>,
) -> Result<()> {
    let queue = &mut ctx.accounts.distribution_queue;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Update available funds from current pool balance
    queue.available_funds = pool.total_pooled;

    // Calculate if we're oversubscribed
    let is_oversubscribed = queue.total_requested_amount > queue.available_funds;
    queue.is_oversubscribed = is_oversubscribed;

    if !is_oversubscribed {
        // Normal case: pay all approved claims
        msg!(
            "Normal distribution: {} claims, {} USDC available, {} USDC requested",
            queue.pending_claims.len(),
            queue.available_funds,
            queue.total_requested_amount
        );
        
        // All pending claims will be paid
        queue.selected_claims = queue.pending_claims.clone();
        
    } else {
        // Oversubscribed: use VRF for fair random selection
        require!(
            randomness.is_some(),
            NovaError::InvalidTimestamp
        );

        let random_bytes = randomness.unwrap();
        queue.vrf_result = Some(random_bytes);

        msg!(
            "Oversubscribed distribution: {} claims, {} USDC available, {} USDC requested",
            queue.pending_claims.len(),
            queue.available_funds,
            queue.total_requested_amount
        );

        // Select claims randomly until we run out of funds
        queue.selected_claims.clear();
        let mut remaining_funds = queue.available_funds;
        let mut selected_indices = Vec::new();

        // Shuffle claims using VRF randomness
        let total_claims = queue.pending_claims.len();
        for i in 0..total_claims {
            if remaining_funds == 0 {
                break;
            }

            // Use different bytes for each selection
            let random_offset = i % 8;
            let random_bytes_subset = [
                random_bytes[random_offset * 4],
                random_bytes[random_offset * 4 + 1],
                random_bytes[random_offset * 4 + 2],
                random_bytes[random_offset * 4 + 3],
            ];
            let random_value = u32::from_le_bytes(random_bytes_subset);

            // Find next unselected claim
            let mut attempts = 0;
            loop {
                let index = ((random_value as usize + attempts) % total_claims) as usize;
                
                if !selected_indices.contains(&index) {
                    selected_indices.push(index);
                    let claim_key = queue.pending_claims[index];
                    
                    // Note: In full implementation, we'd load each claim to check amount
                    // For MVP, we assume average claim size and select proportionally
                    let avg_claim_size = queue.total_requested_amount / total_claims as u64;
                    
                    if remaining_funds >= avg_claim_size {
                        queue.selected_claims.push(claim_key);
                        remaining_funds = remaining_funds.saturating_sub(avg_claim_size);
                    }
                    break;
                }
                
                attempts += 1;
                if attempts >= total_claims {
                    break;
                }
            }
        }

        msg!(
            "Selected {} out of {} claims for payment",
            queue.selected_claims.len(),
            queue.pending_claims.len()
        );
    }

    // Update distribution tracking
    queue.distribution_round = queue
        .distribution_round
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;
    queue.last_distribution = clock.unix_timestamp;

    emit!(ClaimsDistributedEvent {
        pool: pool.key(),
        round: queue.distribution_round,
        total_claims: queue.pending_claims.len() as u32,
        selected_claims: queue.selected_claims.len() as u32,
        oversubscribed: is_oversubscribed,
        available_funds: queue.available_funds,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Payout individual claim (called after distribute_claims selects winners)
pub fn payout_claim(ctx: Context<PayoutClaim>) -> Result<()> {
    let claim = &mut ctx.accounts.claim_request;
    let pool = &mut ctx.accounts.pool;
    let queue = &mut ctx.accounts.distribution_queue;
    let clock = Clock::get()?;

    // Verify claim is approved and selected for payout
    require!(
        claim.status == ClaimStatus::Approved,
        NovaError::InactiveCoverage
    );

    require!(
        queue.selected_claims.contains(&claim.key()),
        NovaError::UnauthorizedValidator
    );

    // Calculate payout amount
    let payout_amount = claim.amount_requested.min(claim.payout_amount.unwrap_or(claim.amount_requested));

    // Verify pool has sufficient funds
    require!(
        pool.total_pooled >= payout_amount,
        NovaError::InsufficientPoolFunds
    );

    // Transfer USDC from pool vault to claimant
    // Extract values needed for seeds before creating CPI context
    let pool_key = pool.key();
    let pool_bump = pool.bump;
    let seeds = &[
        b"vault",
        pool_key.as_ref(),
        &[pool_bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.pool_vault.to_account_info(),
            to: ctx.accounts.claimant_token_account.to_account_info(),
            authority: ctx.accounts.pool_vault.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_ctx, payout_amount)?;

    // Update pool and claim state
    pool.total_pooled = pool.total_pooled.saturating_sub(payout_amount);
    pool.active_claims = pool.active_claims.saturating_sub(1);
    
    claim.status = ClaimStatus::Distributed;
    claim.resolved_at = Some(clock.unix_timestamp);
    claim.payout_amount = Some(payout_amount);

    // Remove from distribution queue
    if let Some(pos) = queue.pending_claims.iter().position(|&c| c == claim.key()) {
        queue.pending_claims.remove(pos);
    }
    if let Some(pos) = queue.selected_claims.iter().position(|&c| c == claim.key()) {
        queue.selected_claims.remove(pos);
    }

    // Update queue totals
    queue.total_approved_claims = queue.total_approved_claims.saturating_sub(1);
    queue.total_requested_amount = queue.total_requested_amount.saturating_sub(payout_amount);

    emit!(ClaimPaidOutEvent {
        claim_id: claim.key(),
        claimant: claim.claimant,
        pool: pool.key(),
        amount: payout_amount,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Claim {} paid out {} USDC to {}",
        claim.key(),
        payout_amount,
        claim.claimant
    );

    Ok(())
}

/// Add approved claim to distribution queue
pub fn add_to_distribution_queue(
    ctx: Context<AddToDistributionQueue>,
) -> Result<()> {
    let queue = &mut ctx.accounts.distribution_queue;
    let claim = &ctx.accounts.claim_request;

    // Verify claim is approved
    require!(
        claim.status == ClaimStatus::Approved,
        NovaError::InactiveCoverage
    );

    // Verify not already in queue
    require!(
        !queue.pending_claims.contains(&claim.key()),
        NovaError::DuplicateValidation
    );

    // Add to queue
    queue.pending_claims.push(claim.key());
    queue.total_approved_claims = queue
        .total_approved_claims
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;
    queue.total_requested_amount = queue
        .total_requested_amount
        .checked_add(claim.amount_requested)
        .ok_or(NovaError::InvalidCoverageAmount)?;

    msg!(
        "Claim {} added to distribution queue. Total: {} claims, {} USDC",
        claim.key(),
        queue.total_approved_claims,
        queue.total_requested_amount
    );

    Ok(())
}

// ============================================================================
// Account Validation Structs
// ============================================================================

#[derive(Accounts)]
pub struct FulfillValidatorSelection<'info> {
    #[account(
        mut,
        seeds = [b"vrf", pool.key().as_ref()],
        bump = vrf_state.bump
    )]
    pub vrf_state: Account<'info, VrfState>,

    #[account(mut)]
    pub claim_request: Account<'info, ClaimRequest>,

    #[account(
        seeds = [b"registry", pool.key().as_ref()],
        bump = validator_registry.bump
    )]
    pub validator_registry: Account<'info, ValidatorRegistry>,

    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeDistributionQueue<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + DistributionQueue::LEN,
        seeds = [b"distribution", pool.key().as_ref()],
        bump
    )]
    pub distribution_queue: Account<'info, DistributionQueue>,

    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeClaims<'info> {
    #[account(
        mut,
        seeds = [b"distribution", pool.key().as_ref()],
        bump = distribution_queue.bump
    )]
    pub distribution_queue: Account<'info, DistributionQueue>,

    #[account(mut)]
    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct PayoutClaim<'info> {
    #[account(mut)]
    pub claim_request: Account<'info, ClaimRequest>,

    #[account(mut)]
    pub pool: Account<'info, InsurancePool>,

    #[account(
        mut,
        seeds = [b"distribution", pool.key().as_ref()],
        bump = distribution_queue.bump
    )]
    pub distribution_queue: Account<'info, DistributionQueue>,

    #[account(
        mut,
        seeds = [b"vault", pool.key().as_ref()],
        bump
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub claimant_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AddToDistributionQueue<'info> {
    #[account(
        mut,
        seeds = [b"distribution", pool.key().as_ref()],
        bump = distribution_queue.bump
    )]
    pub distribution_queue: Account<'info, DistributionQueue>,

    pub claim_request: Account<'info, ClaimRequest>,

    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct ValidatorSelectionFulfilledEvent {
    pub claim_id: Pubkey,
    pub validators: Vec<Pubkey>,
    pub randomness: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct DistributionQueueInitializedEvent {
    pub pool: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct ClaimsDistributedEvent {
    pub pool: Pubkey,
    pub round: u64,
    pub total_claims: u32,
    pub selected_claims: u32,
    pub oversubscribed: bool,
    pub available_funds: u64,
    pub timestamp: i64,
}

#[event]
pub struct ClaimPaidOutEvent {
    pub claim_id: Pubkey,
    pub claimant: Pubkey,
    pub pool: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}
