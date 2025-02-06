use anchor_lang::error_code;


#[error_code]
pub enum AmmError {
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("invalid amount")]
    InvalidAmount,
}
