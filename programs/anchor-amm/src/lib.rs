use anchor_lang::prelude::*;

pub mod state;
pub mod context;
pub mod amm_error;



declare_id!("EwXDx5TcTyKHHGhhyXy1G3x97y785kXYBDe3beiDbqgY");

#[program]
pub mod anchor_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
