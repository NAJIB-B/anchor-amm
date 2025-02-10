use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenInterface, TokenAccount}};

use crate::state::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub mint_x: InterfaceAccount<'info, Mint>,
    pub mint_y: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump,
        space = 8 + Config::INIT_SPACE
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"lp_mint", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>

}


impl<'info> Initialize<'info> {
    pub fn init(&mut self, seed: u64, fee: u16, authority: Option<Pubkey>, bumps: &InitializeBumps) -> Result<()> {
        self.config.set_inner(Config{
            seed,
            authority,
            mint_x: self.mint_x.key(), 
            mint_y: self.mint_y.key(), 
            lp_bump: bumps.lp_mint,
            config_bump: bumps.config,
            fee,
            locked: false

        });
        Ok(())
    }
}
