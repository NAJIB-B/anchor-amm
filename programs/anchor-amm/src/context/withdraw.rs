use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

use constant_product_curve::ConstantProduct;

use crate::state::Config;
use crate::amm_error::AmmError;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: InterfaceAccount<'info, Mint>,
    pub mint_y: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_ata_x: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = user
    )]
    pub user_ata_y: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: InterfaceAccount<'info, TokenAccount>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,
    #[account(
        associated_token::mint = lp_mint,
        associated_token::authority = user
    )]
    pub user_lp_ata: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64, min_x: u64, min_y: u64) -> Result<()> {

        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);
        require!(min_x != 0 || min_y != 0, AmmError::InvalidAmount);


        let result = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
            amount,
            6,
        )
        .unwrap();

        self.withdraw_token(true, result.x)?;
        self.withdraw_token(false, result.y)?;
        self.burn_lp_tokens(amount)?;
        Ok(())
    }
    fn withdraw_token(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_ata_x.to_account_info(),
                self.mint_x.to_account_info(),
                self.mint_x.decimals,
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_ata_y.to_account_info(),
                self.mint_y.to_account_info(),
                self.mint_y.decimals,
            ),
        };

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = TransferChecked {
            from,
            to,
            mint,
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer_checked(cpi_ctx, amount, decimals)?;

        Ok(())
    }

    fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_account = Burn {
            mint: self.lp_mint.to_account_info(),
            from: self.user_lp_ata.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_account);

        burn(cpi_ctx, amount)?;

        Ok(())
    }
}
