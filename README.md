# Anchor AMM

This project implements an Automated Market Maker (AMM) on the Solana blockchain using the Anchor framework, enabling secure and efficient token swaps and liquidity pool management.

---

## Overview

This AMM allows users to:

- Create and manage liquidity pools for different token pairs.
- Swap tokens with minimal slippage, and slippage protection.

## Let's walk through the architecture:

For this program, we will have one state account:
- A Config account


### A Config account consists of:
```rust
#[account]
pub struct Config {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub lp_bump: u8,
    pub config_bump: u8,
    pub fee: u16,
    pub locked: bool
}
```
### In this state account, we will store:

-    seed: A unique identifier to differentiate between various pool configurations.
-    authority: An optional admin key that can lock the pool.
-    mint_x: The first token in the pool.
-    mint_y: The second token in the pool.
-    lp_bump: The bump for the Liquidity Provider PDA.
-    config_bump: The bump for creating the Config account PDA.
-    fee: The fee applied to each swap in the pool, in basis points.
-    locked: Whether the pool is locked.

### The pool creator will be able to create a Config for a liquidity pool. For that, we create the Initialize context
```rust

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
```

Let´s have a closer look at the accounts that we are passing in this context:

- initializer: The user who signs and pays for the initialization of new Config. This must be mutable as lamports will be deducted.
- mint_x, mint_y: The mint accounts for the two tokens that will be part of the liquidity pool.
- lp_mint: A newly created mint for the liquidity provider (LP) tokens. It's initialized with 6 decimals and its authority is set to the config account, ensuring the pool controls minting.
- vault_x, vault_y: These are token accounts associated with mint_x and mint_y respectively, where tokens will be stored. They are owned by the authority account, acting as vaults for the pool's assets.
- config: This is the Config account for the pool, initialized with a seed for uniqueness across different pools. It holds all the pool's configuration data like fees, token mints, and bumps for PDAs.
- associated_token_program: to initialize vault accounts
- token_program: to the lp_mint account
- system_program: to initialize config account

### We implement simple initialize functionality for the Initialize context:

```rust
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
```

We simply initialize the LP Config account with its initial data.


---

### Users will be able to deposit tokens into the liquidity pool. For that, we create the following context:
```rust
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
```
Accounts in the Deposit Context:

- user: The user who signs the transaction to deposit tokens into the pool. This account is mutable because it has to pay for account creation.
- mint_x, mint_y: Mint accounts for the tokens being deposited. These ensure the correct tokens are being used in the pool.
- config: The configuration account for the pool, checked to ensure it's the right pool for the tokens being deposited. The has_one constraint verifies that mint_x and mint_y match the pool's setup.
- lp_mint: The mint for LP (Liquidity Provider) tokens, which are minted when users deposit into the pool. This account is mutable as new LP tokens will be minted.
- vault_x, vault_y: These are the pool's vaults where deposited tokens are stored. They are mutable because tokens will be added to these accounts. 
- user_ata_x, user_ata_y: The user's token accounts from which tokens will be transferred to the pool's vaults. These accounts are mutable due to the token transfer.
- user_lp_ata: The user's LP token account. If it doesn't exist, it's initialized here. This account is mutable because new LP tokens are deposited here.
- token_program, system_program, associated_token_program: Necessary programs for token transfers, account creation, and managing associated token accounts.

### We then implement the deposit functionality for the Deposit context:
```rust
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
```

In this implementation we start by calculating the tokens to be deposited into the pool. In case of an empty pool, we use the arguments from the user. If the pool already contains tokens, we use the ConstantProduct::xy_deposit_amounts_from_l() function from an external crate to calculate the proportinate amount of tokens to deposit into the pool, while we control the slippage to protect the user.

---

### Users can also withdraw tokens from the liquidity pool:
Accounts in the Withdraw Context are similar to the accounts in the deposit context:

- user: The user initiating the withdrawal.
- mint_x, mint_y: Mint accounts for the tokens in the pool, ensuring the withdrawal involves the correct tokens.
- config: The configuration account for the pool, validated to match the pool from which tokens are being withdrawn. The has_one constraints check if mint_x and mint_y are correct.
- lp_mint: The mint for LP tokens, mutable as supply has to be adjusted during withdraval.
- vault_x, vault_y: Pool vaults from which tokens will be withdrawn, mutable for token transfer.
- user_ata_x, user_ata_y: User's token accounts where withdrawn tokens will be sent. Initialized if they don't exist yet, mutable for receiving tokens.
- user_lp_ata: The user's LP token account from which LP tokens will be burned. Mutable as tokens are removed from here during withdrawal.
- token_program, system_program, associated_token_program: Programs required for token operations, account management, and token burning.


### We then implement the withdraw functionality for the Withdraw context:

```rust
impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64, min_x: u64, min_y: u64) -> Result<()> {

        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);
        require!(min_x != 0 || min_y != 0, AmmError::InvalidAmount);


        let ammounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
            amount,
            6,
        )
        .map_err(AmmError::from)?;

        require!(min_x < ammounts.x && min_y < ammounts.y, AmmError::SlippageExceeded);



        self.withdraw_token(true, ammounts.x)?;
        self.withdraw_token(false, ammounts.y)?;
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
```

withdraw: Burns LP tokens and transfers corresponding tokens back to the user, checking for pool lock status and slippage protection.
withdraw_token: Transfers tokens from pool vaults to user.
burn_lp_tokens: Burns LP tokens from the user's account, reducing LP token supply.


### Users will be able to swap tokens using the liquidity pool. For that, we create the following context:

```rust
#[derive(Accounts)]

pub struct Swap<'info> {
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
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
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
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```
This context setup ensures that the swap operation can be securely executed, maintaining the integrity of both the user's assets and the pool's reserves.

- user: The user performing the swap, mutable because this account will pay for any necessary account initialization and token transfers.
- mint_x, mint_y: Mint accounts for the tokens involved in the swap, ensuring the correct tokens are used in the transaction.
- user_ata_x, user_ata_y: User's token accounts for tokens X and Y respectively. These accounts are initialized if needed, using the user as payer. They hold the tokens before and after the swap.
- vault_x, vault_y: The pool's vault accounts where tokens X and Y are stored. These accounts are mutable as tokens will be moved in or out during the swap.
- config: The configuration account for the liquidity pool, checked to ensure it matches the tokens being swapped. The has_one constraints verify that mint_x and mint_y match the pool's setup.
- token_program: The Solana Token program, necessary for handling token transfers during the swap.
- associated_token_program: Used to initialize the user's token accounts if they do not exist, ensuring proper token account management.
- system_program: Required for creating or initializing accounts on Solana, including any new associated token accounts.

### We then implement some functionality for this context:
```rust
impl<'info> Swap<'info> {
    pub fn swap(&mut self, amount: u64, min: u64, is_x: bool) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.vault_x.amount,
            self.config.fee,
            None,
        )
        .map_err(AmmError::from)?;

        let p = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        let res = curve.swap(p, amount, min).map_err(AmmError::from)?;

        require!(res.deposit != 0, AmmError::InvalidAmount);
        require!(res.withdraw != 0, AmmError::InvalidAmount);

        // deposit tokens
        self.deposit_tokens(is_x, res.deposit)?;
        // withdraw tokens
        self.withdraw_tokens(is_x, res.withdraw)?;
        // transfer fee
        Ok(())
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = match is_x {
            true => (
                self.user_ata_x.to_account_info(),
                self.vault_x.to_account_info(),
                self.mint_x.to_account_info(),
                self.mint_x.decimals,
            ),
            false => (
                self.user_ata_y.to_account_info(),
                self.vault_y.to_account_info(),
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

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
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
}

```

Here we check if the pool is locked, walidation if the swap amount is not invalid, and initializing a ConstantProduct curve with the current state of the pool to calculate the amount of tokens the user should receive from the swap.
Then we deposit the tokens from the user based on token type, and return the other token the user wanted to receive.
