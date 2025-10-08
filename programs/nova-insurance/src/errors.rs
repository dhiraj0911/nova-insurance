use anchor_lang::prelude::*;

#[error_code]
pub enum NovaError {
    #[msg("Premium amount must be greater than zero")]
    InvalidPremiumAmount,
    
    #[msg("Coverage amount must be greater than premium amount")]
    InvalidCoverageAmount,
    
    #[msg("Minimum validators must be at least 3")]
    InsufficientValidators,
    
    #[msg("Claim period must be positive")]
    InvalidClaimPeriod,
    
    #[msg("Pool type is invalid")]
    InvalidPoolType,
    
    #[msg("Coverage amount exceeds pool maximum")]
    ExcessiveCoverageAmount,
    
    #[msg("User coverage is not active")]
    InactiveCoverage,
    
    #[msg("Claim amount exceeds user coverage")]
    ExcessiveClaimAmount,
    
    #[msg("Claim period has expired")]
    ClaimPeriodExpired,
    
    #[msg("Insufficient pool funds for claim")]
    InsufficientPoolFunds,
    
    #[msg("Validator is not authorized for this claim")]
    UnauthorizedValidator,
    
    #[msg("Validator has already validated this claim")]
    DuplicateValidation,
    
    #[msg("Stake amount is below minimum required")]
    InsufficientStake,
    
    #[msg("Validator reputation is too low")]
    LowReputation,
    
    #[msg("Premium payment is overdue")]
    PremiumOverdue,
    
    #[msg("Mathematical overflow occurred")]
    MathOverflow,
    
    #[msg("Unauthorized access attempt")]
    Unauthorized,
    
    #[msg("Account is already initialized")]
    AlreadyInitialized,
    
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
}
