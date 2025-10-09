use anchor_lang::prelude::*;

/// Pool types for different insurance categories
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoolType {
    Medical,
    Weather,
    Crop,
    General,
}

impl Space for PoolType {
    const INIT_SPACE: usize = 1; // enum discriminant
}

/// Incident types for claims
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum IncidentType {
    MedicalEmergency,
    NaturalDisaster,
    Accident,
    CropFailure,
    PropertyDamage,
    Other,
}

impl Space for IncidentType {
    const INIT_SPACE: usize = 1; // enum discriminant
}

/// Claim status tracking
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClaimStatus {
    Pending,
    UnderValidation,
    Approved,
    Rejected,
    Distributed,
    Queued,
}

impl Space for ClaimStatus {
    const INIT_SPACE: usize = 1; // enum discriminant
}

/// Individual validation record
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Validation {
    pub validator: Pubkey,
    pub approved: bool,
    pub reason: String,
    pub timestamp: i64,
}

impl Space for Validation {
    const INIT_SPACE: usize = 32 + 1 + 4 + 200 + 8; // validator + approved + string len + reason (max 200) + timestamp
}

/// Main insurance pool account
/// Holds all configuration and state for a specific insurance pool
#[account]
#[derive(InitSpace)]
pub struct InsurancePool {
    /// Unique identifier for this pool
    pub pool_id: Pubkey,
    
    /// Type of insurance coverage this pool provides
    pub pool_type: PoolType,
    
    /// Authority that can manage this pool
    pub authority: Pubkey,
    
    /// USDC vault address where premiums are stored
    pub vault: Pubkey,
    
    /// Monthly premium amount in USDC (lamports)
    pub premium_amount: u64,
    
    /// Maximum coverage amount per user in USDC (lamports)
    pub coverage_amount: u64,
    
    /// Total USDC currently pooled
    pub total_pooled: u64,
    
    /// Number of active members in the pool
    pub total_members: u32,
    
    /// Number of active claims being processed
    pub active_claims: u32,
    
    /// Time window for claims (in seconds)
    pub claim_period: i64,
    
    /// Minimum number of validators required for claim verification
    pub min_validators: u8,
    
    /// Timestamp when pool was created
    pub created_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl InsurancePool {
    /// Calculate space needed for InsurancePool account
    pub const LEN: usize = 8 + // discriminator
        32 + // pool_id
        1 + // pool_type (enum)
        32 + // authority
        32 + // vault
        8 + // premium_amount
        8 + // coverage_amount
        8 + // total_pooled
        4 + // total_members
        4 + // active_claims
        8 + // claim_period
        1 + // min_validators
        8 + // created_at
        1; // bump
}

/// User coverage account tracking individual member's insurance status
#[account]
#[derive(InitSpace)]
pub struct UserCoverage {
    /// User's wallet address
    pub user: Pubkey,
    
    /// Insurance pool this coverage belongs to
    pub pool: Pubkey,
    
    /// Total premiums paid by this user
    pub premiums_paid: u64,
    
    /// Timestamp of last premium payment
    pub last_payment: i64,
    
    /// Whether coverage is currently active
    pub coverage_active: bool,
    
    /// Amount of coverage this user has
    pub coverage_amount: u64,
    
    /// Number of claims made by this user
    pub claims_made: u8,
    
    /// Timestamp when user joined the pool
    pub joined_at: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl UserCoverage {
    /// Calculate space needed for UserCoverage account
    pub const LEN: usize = 8 + // discriminator
        32 + // user
        32 + // pool
        8 + // premiums_paid
        8 + // last_payment
        1 + // coverage_active
        8 + // coverage_amount
        1 + // claims_made
        8 + // joined_at
        1; // bump
}

/// Validator stake account for community claim validators
#[account]
#[derive(InitSpace)]
pub struct ValidatorStake {
    /// Validator's wallet address
    pub validator: Pubkey,
    
    /// Amount of SOL staked by validator
    pub stake_amount: u64,
    
    /// Total number of validations completed
    pub validations_completed: u32,
    
    /// Number of successful validations (correct decisions)
    pub successful_validations: u32,
    
    /// Reputation score (0-10000 scale)
    pub reputation_score: u32,
    
    /// Timestamp of last validation
    pub last_validation: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl ValidatorStake {
    /// Calculate space needed for ValidatorStake account
    pub const LEN: usize = 8 + // discriminator
        32 + // validator
        8 + // stake_amount
        4 + // validations_completed
        4 + // successful_validations
        4 + // reputation_score
        8 + // last_validation
        1; // bump
    
    /// Initial reputation score for new validators
    pub const INITIAL_REPUTATION: u32 = 5000;
    
    /// Maximum reputation score
    pub const MAX_REPUTATION: u32 = 10000;
}

/// Validator registry for a pool - tracks all validators
#[account]
#[derive(InitSpace)]
pub struct ValidatorRegistry {
    /// The pool this registry belongs to
    pub pool: Pubkey,
    
    /// List of active validators (max 100)
    #[max_len(100)]
    pub validators: Vec<Pubkey>,
    
    /// Total number of validators
    pub total_validators: u32,
    
    /// PDA bump seed
    pub bump: u8,
}

impl ValidatorRegistry {
    /// Calculate space needed for ValidatorRegistry account
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        4 + (32 * 100) + // validators (vec + max 100 pubkeys)
        4 + // total_validators
        1; // bump
}

/// VRF state for random validator selection
#[account]
#[derive(InitSpace)]
pub struct VrfState {
    /// The pool this VRF state belongs to
    pub pool: Pubkey,
    
    /// Switchboard VRF account
    pub switchboard_vrf: Pubkey,
    
    /// Authority for VRF requests
    pub authority: Pubkey,
    
    /// Last randomness result
    pub last_randomness: Option<[u8; 32]>,
    
    /// Last timestamp VRF was called
    pub last_timestamp: i64,
    
    /// Pending claims awaiting validator assignment (max 50)
    #[max_len(50)]
    pub pending_claims: Vec<Pubkey>,
    
    /// Total VRF requests completed
    pub requests_completed: u64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl VrfState {
    /// Calculate space needed for VrfState account
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        32 + // switchboard_vrf
        32 + // authority
        1 + 32 + // last_randomness (option + 32 bytes)
        8 + // last_timestamp
        4 + (32 * 50) + // pending_claims (vec + max 50 pubkeys)
        8 + // requests_completed
        1; // bump
}

/// Claim request account for insurance claims
#[account]
#[derive(InitSpace)]
pub struct ClaimRequest {
    /// Unique claim identifier
    pub claim_id: Pubkey,
    
    /// User making the claim
    pub claimant: Pubkey,
    
    /// Insurance pool for this claim
    pub pool: Pubkey,
    
    /// Amount requested in USDC
    pub amount_requested: u64,
    
    /// Type of incident
    pub incident_type: IncidentType,
    
    /// Timestamp of the incident
    pub incident_timestamp: i64,
    
    /// IPFS hash of claim documentation (max 100 chars)
    #[max_len(100)]
    pub description: String,
    
    /// Validators assigned via VRF (max 10 validators)
    #[max_len(10)]
    pub validators_assigned: Vec<Pubkey>,
    
    /// Validation records (max 10 validations)
    #[max_len(10)]
    pub validations: Vec<Validation>,
    
    /// Number of approvals received
    pub approvals: u8,
    
    /// Number of rejections received
    pub rejections: u8,
    
    /// Current status of the claim
    pub status: ClaimStatus,
    
    /// VRF result used for validator selection
    pub vrf_result: Option<[u8; 32]>,
    
    /// Timestamp when claim was created
    pub created_at: i64,
    
    /// Timestamp when claim was resolved
    pub resolved_at: Option<i64>,
    
    /// Actual payout amount (may differ from requested)
    pub payout_amount: Option<u64>,
    
    /// PDA bump seed
    pub bump: u8,
}

impl ClaimRequest {
    /// Calculate space needed for ClaimRequest account
    pub const LEN: usize = 8 + // discriminator
        32 + // claim_id
        32 + // claimant
        32 + // pool
        8 + // amount_requested
        1 + // incident_type
        8 + // incident_timestamp
        4 + 100 + // description (vec + max 100 chars)
        4 + (32 * 10) + // validators_assigned (vec + max 10 pubkeys)
        4 + (245 * 10) + // validations (vec + max 10 validations)
        1 + // approvals
        1 + // rejections
        1 + // status
        1 + 32 + // vrf_result (option + 32 bytes)
        8 + // created_at
        1 + 8 + // resolved_at (option + i64)
        1 + 8 + // payout_amount (option + u64)
        1; // bump
}

/// Distribution queue for managing oversubscribed claims
/// When total approved claims exceed pool funds, this manages fair distribution
#[account]
#[derive(InitSpace)]
pub struct DistributionQueue {
    /// The pool this queue belongs to
    pub pool: Pubkey,
    
    /// Total approved claims awaiting distribution
    pub total_approved_claims: u32,
    
    /// Total amount requested by approved claims
    pub total_requested_amount: u64,
    
    /// Available funds in pool for distribution
    pub available_funds: u64,
    
    /// List of approved claims pending payout (max 100)
    #[max_len(100)]
    pub pending_claims: Vec<Pubkey>,
    
    /// Claims selected for payout in current round (max 50)
    #[max_len(50)]
    pub selected_claims: Vec<Pubkey>,
    
    /// VRF result for current distribution round
    pub vrf_result: Option<[u8; 32]>,
    
    /// Whether distribution is oversubscribed
    pub is_oversubscribed: bool,
    
    /// Current distribution round number
    pub distribution_round: u64,
    
    /// Timestamp of last distribution
    pub last_distribution: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl DistributionQueue {
    /// Calculate space needed for DistributionQueue account
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        4 + // total_approved_claims
        8 + // total_requested_amount
        8 + // available_funds
        4 + (32 * 100) + // pending_claims (vec + max 100 pubkeys)
        4 + (32 * 50) + // selected_claims (vec + max 50 pubkeys)
        1 + 32 + // vrf_result (option + 32 bytes)
        1 + // is_oversubscribed
        8 + // distribution_round
        8 + // last_distribution
        1; // bump
}

/// Priority scoring for claim distribution (basic structure for future enhancement)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PriorityScore {
    /// Base priority score
    pub base_score: u32,
    
    /// User's premium payment history multiplier
    pub payment_history_score: u32,
    
    /// Claim urgency factor
    pub urgency_score: u32,
    
    /// Time in queue factor
    pub time_in_queue_score: u32,
    
    /// Total calculated priority
    pub total_score: u32,
}

impl Space for PriorityScore {
    const INIT_SPACE: usize = 4 + 4 + 4 + 4 + 4; // 5 u32 fields
}

impl PriorityScore {
    /// Calculate basic priority score
    pub fn calculate(
        premiums_paid: u64,
        time_in_queue: i64,
        incident_type: IncidentType,
    ) -> Self {
        // Base score from incident type
        let base_score = match incident_type {
            IncidentType::MedicalEmergency => 1000,
            IncidentType::NaturalDisaster => 800,
            IncidentType::Accident => 600,
            IncidentType::CropFailure => 500,
            IncidentType::PropertyDamage => 400,
            IncidentType::Other => 300,
        };
        
        // Payment history score (more premiums = higher priority)
        let payment_history_score = (premiums_paid / 1_000_000).min(500) as u32; // Cap at 500
        
        // Time in queue score (longer wait = higher priority)
        let time_in_queue_score = (time_in_queue / 86400).min(200) as u32; // 1 point per day, cap at 200
        
        // Urgency score (currently same as base, can be enhanced)
        let urgency_score = base_score / 2;
        
        let total_score = base_score + payment_history_score + time_in_queue_score + urgency_score;
        
        Self {
            base_score,
            payment_history_score,
            urgency_score,
            time_in_queue_score,
            total_score,
        }
    }
}
