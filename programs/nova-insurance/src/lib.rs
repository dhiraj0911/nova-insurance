use anchor_lang::prelude::*;

declare_id!("DB1ZyxKho5hwQPd6r7C1FSTifw5N7G5YYh5gyhvcpGN5");

pub mod errors;
pub mod state;
pub mod instructions;

#[allow(unused_imports)]
use errors::*;
#[allow(unused_imports)]
use state::*;
use instructions::*;

#[program]
pub mod nova_insurance {
    use super::*;

    /// Initialize a new insurance pool
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        pool_type: PoolType,
        premium_amount: u64,
        coverage_amount: u64,
        min_validators: u8,
        claim_period: i64,
    ) -> Result<()> {
        instructions::initialize_pool(
            ctx,
            pool_type,
            premium_amount,
            coverage_amount,
            min_validators,
            claim_period,
        )
    }

    /// Join an existing insurance pool
    pub fn join_pool(ctx: Context<JoinPool>, coverage_amount: u64) -> Result<()> {
        instructions::join_pool(ctx, coverage_amount)
    }

    /// Pay monthly premium to maintain coverage
    pub fn pay_premium(ctx: Context<PayPremium>) -> Result<()> {
        instructions::pay_premium(ctx)
    }

    /// Submit a new insurance claim
    pub fn submit_claim(
        ctx: Context<SubmitClaim>,
        incident_type: IncidentType,
        amount_requested: u64,
        incident_timestamp: i64,
        description: String,
    ) -> Result<()> {
        instructions::submit_claim(
            ctx,
            incident_type,
            amount_requested,
            incident_timestamp,
            description,
        )
    }

    /// Stake SOL to become a validator
    pub fn stake_as_validator(
        ctx: Context<StakeAsValidator>,
        stake_amount: u64,
    ) -> Result<()> {
        instructions::stake_as_validator(ctx, stake_amount)
    }

    /// Validate a claim (approve or reject)
    pub fn validate_claim(
        ctx: Context<ValidateClaim>,
        approve: bool,
        reason: String,
    ) -> Result<()> {
        instructions::validate_claim(ctx, approve, reason)
    }

    /// Initialize VRF state for a pool
    pub fn initialize_vrf_state(ctx: Context<InitializeVrfState>) -> Result<()> {
        instructions::initialize_vrf_state(ctx)
    }

    /// Request validator selection using VRF
    pub fn request_validator_selection(
        ctx: Context<RequestValidatorSelection>,
        claim_id: Pubkey,
    ) -> Result<()> {
        instructions::request_validator_selection(ctx, claim_id)
    }

    /// Fulfill VRF callback for validator selection
    pub fn fulfill_validator_selection(
        ctx: Context<FulfillValidatorSelection>,
        randomness: [u8; 32],
    ) -> Result<()> {
        instructions::fulfill_validator_selection(ctx, randomness)
    }

    /// Initialize distribution queue for a pool
    pub fn initialize_distribution_queue(
        ctx: Context<InitializeDistributionQueue>,
    ) -> Result<()> {
        instructions::initialize_distribution_queue(ctx)
    }

    /// Add approved claim to distribution queue
    pub fn add_to_distribution_queue(
        ctx: Context<AddToDistributionQueue>,
    ) -> Result<()> {
        instructions::add_to_distribution_queue(ctx)
    }

    /// Distribute claims (normal or oversubscribed)
    pub fn distribute_claims(
        ctx: Context<DistributeClaims>,
        randomness: Option<[u8; 32]>,
    ) -> Result<()> {
        instructions::distribute_claims(ctx, randomness)
    }

    /// Payout individual claim
    pub fn payout_claim(ctx: Context<PayoutClaim>) -> Result<()> {
        instructions::payout_claim(ctx)
    }

    /// Deposit idle pool funds to yield vault (Kamino)
    pub fn deposit_to_yield(ctx: Context<DepositToYield>, amount: u64) -> Result<()> {
        instructions::deposit_to_yield(ctx, amount)
    }

    /// Withdraw funds from yield vault back to pool
    pub fn withdraw_from_yield(ctx: Context<WithdrawFromYield>, amount: u64) -> Result<()> {
        instructions::withdraw_from_yield(ctx, amount)
    }
}
