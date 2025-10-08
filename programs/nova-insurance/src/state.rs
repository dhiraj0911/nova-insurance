use anchor_lang::prelude::*;

/// Pool types for different insurance categories
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum PoolType {
    Medical,
    Weather,
    Crop,
}

/// Main insurance pool account
/// Holds all configuration and state for a specific insurance pool
#[account]
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
        8 + // claim_period
        1 + // min_validators
        8 + // created_at
        1; // bump
}

/// User coverage account tracking individual member's insurance status
#[account]
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
