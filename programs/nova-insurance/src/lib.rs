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
}
