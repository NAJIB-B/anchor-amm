use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{TransferChecked, transfer_checked, Mint, TokenInterface, TokenAccount, MintTo, mint_to}};

use constant_product_curve::ConstantProduct;

use crate::state::Config;
use crate::amm_error::AmmError;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: InterfaceAccount<'info, Mint>,
    pub mint_y: InterfaceAccount<'info, Mint>,
    #[account(
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_ata_x: InterfaceAccount<'info, TokenAccount>,
    #[account(
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
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user
    )]
    pub user_lp_ata: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
    
}


impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        let (x, y) = match self.lp_mint.supply == 0 && self.vault_x.amount == 0 && self.vault_y.amount == 0 {
            true => (max_x, max_y),
            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.lp_mint.supply,
                    amount,
                    6
                ).unwrap();
                (amounts.x, amounts.y)
            },
        };

        self.deposit_token(true, x)?;
        self.deposit_token(false, y)?;

        self.mint_lp_tokens(amount)?;

        Ok(())
    }
    fn deposit_token(&mut self, is_x: bool, amount: u64) -> Result<()> {

        let (from, to, mint, decimals) = match is_x {
            true => (self.user_ata_x.to_account_info(), self.vault_x.to_account_info(), self.mint_x.to_account_info(), self.mint_x.decimals),
            false => (self.user_ata_y.to_account_info(), self.vault_y.to_account_info(), self.mint_y.to_account_info(), self.mint_y.decimals)
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

    fn mint_lp_tokens(&self, amount: u64) -> Result<()> {

        let cpi_program = self.token_program.to_account_info();

        let cpi_account = MintTo {
            mint: self.lp_mint.to_account_info(),
            to: self.user_lp_ata.to_account_info(),
            authority: self.config.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]; 1] = &[&[
            b"config",
            &self.config.seed.to_le_bytes()[..],
            &[self.config.config_bump]
        ]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_account, signer_seeds);

        mint_to(cpi_ctx, amount)?;

        Ok(())
    }
}
