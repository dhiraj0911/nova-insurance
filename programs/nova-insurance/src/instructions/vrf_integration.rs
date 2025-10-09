use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;

use crate::errors::*;
use crate::state::*;

/// Initialize VRF state for a pool
pub fn initialize_vrf_state(
    ctx: Context<InitializeVrfState>,
) -> Result<()> {
    let vrf_state = &mut ctx.accounts.vrf_state;
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    vrf_state.pool = pool.key();
    vrf_state.switchboard_vrf = Pubkey::default(); // Will be set when Switchboard is integrated
    vrf_state.authority = ctx.accounts.authority.key();
    vrf_state.last_randomness = None;
    vrf_state.last_timestamp = clock.unix_timestamp;
    vrf_state.pending_claims = Vec::new();
    vrf_state.requests_completed = 0;
    vrf_state.bump = *ctx.bumps.get("vrf_state").unwrap();

    emit!(VrfStateInitializedEvent {
        pool: pool.key(),
        timestamp: clock.unix_timestamp,
    });

    msg!("VRF state initialized for pool {}", pool.key());

    Ok(())
}

/// Request validator selection (simplified for MVP without Switchboard)
pub fn request_validator_selection(
    ctx: Context<RequestValidatorSelection>,
    claim_id: Pubkey,
) -> Result<()> {
    let vrf_state = &mut ctx.accounts.vrf_state;
    let claim = &mut ctx.accounts.claim_request;
    let pool = &ctx.accounts.pool;
    let validator_registry = &ctx.accounts.validator_registry;
    let clock = Clock::get()?;

    // Verify claim is pending and needs validators
    require!(
        claim.status == ClaimStatus::Pending,
        NovaError::ClaimPeriodExpired
    );

    // Verify claim belongs to this pool
    require!(
        claim.pool == pool.key(),
        NovaError::InactiveCoverage
    );

    // Verify validators not already assigned
    require!(
        claim.validators_assigned.is_empty(),
        NovaError::DuplicateValidation
    );

    // Check we have enough validators in the registry
    require!(
        validator_registry.validators.len() >= pool.min_validators as usize,
        NovaError::InsufficientValidators
    );

    // Generate pseudo-randomness for MVP (deterministic but unpredictable)
    // In production, this would use Switchboard VRF
    let randomness = generate_randomness(
        &claim_id,
        &pool.key(),
        clock.unix_timestamp,
        clock.slot,
    );

    // Select validators using the randomness
    let selected_validators = select_random_validators(
        &randomness,
        &validator_registry.validators,
        pool.min_validators as usize,
    )?;

    // Assign validators to claim
    claim.validators_assigned = selected_validators.clone();
    claim.status = ClaimStatus::UnderValidation;
    claim.vrf_result = Some(randomness);

    // Update VRF state
    vrf_state.last_randomness = Some(randomness);
    vrf_state.last_timestamp = clock.unix_timestamp;
    vrf_state.requests_completed = vrf_state
        .requests_completed
        .checked_add(1)
        .ok_or(NovaError::InvalidCoverageAmount)?;

    emit!(ValidatorsAssignedEvent {
        pool: pool.key(),
        claim_id,
        validators: selected_validators,
        randomness,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Assigned {} validators to claim {}",
        pool.min_validators,
        claim_id
    );

    Ok(())
}

/// Generate pseudo-randomness for validator selection
/// Note: This is deterministic but unpredictable for MVP
/// Production should use Switchboard VRF for true randomness
fn generate_randomness(
    claim_id: &Pubkey,
    pool_id: &Pubkey,
    timestamp: i64,
    slot: u64,
) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(claim_id.as_ref());
    data.extend_from_slice(pool_id.as_ref());
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&slot.to_le_bytes());
    
    let hash_result = hash(&data);
    hash_result.to_bytes()
}

/// Select random validators from available pool
fn select_random_validators(
    randomness: &[u8; 32],
    available_validators: &[Pubkey],
    num_required: usize,
) -> Result<Vec<Pubkey>> {
    require!(
        available_validators.len() >= num_required,
        NovaError::InsufficientValidators
    );

    let mut selected = Vec::new();
    let mut used_indices = Vec::new();

    for i in 0..num_required {
        // Use different bytes of randomness for each selection
        let start_byte = (i * 4) % 28; // Ensure we stay within bounds
        let index_seed = u32::from_le_bytes([
            randomness[start_byte],
            randomness[start_byte + 1],
            randomness[start_byte + 2],
            randomness[start_byte + 3],
        ]);

        let mut index = (index_seed as usize) % available_validators.len();
        
        // Ensure no duplicates by finding next unused validator
        while used_indices.contains(&index) {
            index = (index + 1) % available_validators.len();
        }

        used_indices.push(index);
        selected.push(available_validators[index]);
    }

    Ok(selected)
}

// ============================================================================
// Account Validation Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializeVrfState<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + VrfState::INIT_SPACE,
        seeds = [b"vrf_state", pool.key().as_ref()],
        bump
    )]
    pub vrf_state: Account<'info, VrfState>,

    pub pool: Account<'info, InsurancePool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestValidatorSelection<'info> {
    #[account(
        mut,
        seeds = [b"vrf_state", pool.key().as_ref()],
        bump = vrf_state.bump,
        constraint = vrf_state.pool == pool.key() @ NovaError::InactiveCoverage
    )]
    pub vrf_state: Account<'info, VrfState>,

    #[account(mut)]
    pub claim_request: Account<'info, ClaimRequest>,

    pub pool: Account<'info, InsurancePool>,

    #[account(
        seeds = [b"validator_registry", pool.key().as_ref()],
        bump = validator_registry.bump,
        constraint = validator_registry.pool == pool.key() @ NovaError::InactiveCoverage
    )]
    pub validator_registry: Account<'info, ValidatorRegistry>,

    pub clock: Sysvar<'info, Clock>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct VrfStateInitializedEvent {
    pub pool: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct ValidatorsAssignedEvent {
    pub pool: Pubkey,
    pub claim_id: Pubkey,
    pub validators: Vec<Pubkey>,
    pub randomness: [u8; 32],
    pub timestamp: i64,
}
