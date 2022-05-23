use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount,};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// Replace for Devnet Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr
// Replace for Localnet 8fFnX9WSPjJEADtG5jQvQQptzfFmmjd6hrW7HjuUT8ur
pub const USDC_MINT_ADDRESS: &str = "8fFnX9WSPjJEADtG5jQvQQptzfFmmjd6hrW7HjuUT8ur";


#[program]
pub mod token3 {
    use super::*;

     pub fn new_token(ctx: Context<NewToken>, name: String, transaction_fee: u64, sale_fee: u64, discount: u64, reward: u64) -> Result<()> {
        //TODO: check pdas match accounts passed
        let (token_pda, token_bump) =
            Pubkey::find_program_address(&["MINT".as_bytes(), ctx.accounts.token_data.key().as_ref()], ctx.program_id);

        let (earned_pda, earned_bump) =
            Pubkey::find_program_address(&["EARNED".as_bytes(), ctx.accounts.token_data.key().as_ref(), ctx.accounts.mint.key().as_ref()], ctx.program_id);
        
        let (reserve_pda, reserve_bump) =
            Pubkey::find_program_address(&["RESERVE".as_bytes(), ctx.accounts.token_data.key().as_ref(), ctx.accounts.mint.key().as_ref()], ctx.program_id);

        if token_pda != ctx.accounts.token_mint.key() {
            return err!(ErrorCode::PDA);
        }

        if earned_pda != ctx.accounts.earned_usdc_account.key() {
            return err!(ErrorCode::PDA);
        }

        if reserve_pda != ctx.accounts.reserve_usdc_account.key() {
            return err!(ErrorCode::PDA);
        }

        let token_data = &mut ctx.accounts.token_data;
        token_data.name = name;
        token_data.user = ctx.accounts.user.key();
        token_data.mint = token_pda;
        token_data.earned = earned_pda;
        token_data.reserve = reserve_pda;
        token_data.mint_bump = token_bump;
        token_data.earned_bump = earned_bump;
        token_data.reserve_bump = reserve_bump;
        token_data.transaction_fee = transaction_fee;
        token_data.sale_fee = sale_fee;
        token_data.discount = discount;
        token_data.reward = reward;
        
        Ok(())
    }

    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        let token_data = ctx.accounts.token_data.key();

        let seeds = &["MINT".as_bytes(), token_data.as_ref(), &[ctx.accounts.token_data.mint_bump]];
        let signer = [&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.token_mint.to_account_info(),
            },
            &signer,
        );
        token::mint_to(cpi_ctx, amount)?;

        let discount = &ctx.accounts.token_data.discount;
        let sale_fee = &ctx.accounts.token_data.sale_fee;
        
        // TODO: Account for Decimals in Mint
        let usdc_amount = amount * (100-discount) / 100;
        let fee_amount = usdc_amount * (sale_fee) / 100;
        let reserve_amount = usdc_amount - fee_amount ;
        
        // transfer USDC from the User to Treasury
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC from the User to Reserve
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.reserve_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, reserve_amount)?;


        Ok(())
    }


    pub fn redeem(ctx: Context<Redeem>, amount: u64,) -> Result<()> {

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.token_mint.to_account_info(),
                from: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)?;
        
        let token_data = ctx.accounts.token_data.key();
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), token_data.as_ref(), mint.as_ref(), &[ctx.accounts.token_data.reserve_bump]];
        let signer = [&seeds[..]];

        // transfer USDC fee.
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
            &signer,
        );

        let fee_amount = ctx.accounts.token_data.transaction_fee; 
        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC earned
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        let usdc_value = amount * (ctx.accounts.reserve_usdc_account.amount) / (ctx.accounts.token_mint.supply);
        let earned_amount = usdc_value - fee_amount;
        token::transfer(cpi_ctx, earned_amount)?;
        
        
        Ok(())
    }

    pub fn partial_redeem(ctx: Context<PartialRedeem>, token_amount: u64, usdc_amount:u64) -> Result<()> {

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.token_mint.to_account_info(),
                from: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, token_amount)?;
        
        let token_data = ctx.accounts.token_data.key();
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), token_data.as_ref(), mint.as_ref(), &[ctx.accounts.token_data.reserve_bump]];
        let signer = [&seeds[..]];

        // transfer USDC fee.
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
            &signer,
        );

        let fee_amount = ctx.accounts.token_data.transaction_fee; 
        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC earned
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        let usdc_value = token_amount * (ctx.accounts.reserve_usdc_account.amount) / (ctx.accounts.token_mint.supply);
        let earned_amount = usdc_value - fee_amount;
        token::transfer(cpi_ctx, earned_amount)?;

        // transfer USDC from the User to Treasury
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, usdc_amount)?;
        
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct NewToken<'info> {
    #[account(
        init,
        payer = user,
        space = 10000 // TODO: calculate space
    )]
    pub token_data: Account<'info, TokenData>,

    #[account(
        init,
        seeds = ["MINT".as_bytes().as_ref(), token_data.key().as_ref()],
        bump,
        payer = user,
        mint::decimals = 6,
        mint::authority = token_mint, 
        
    )]
    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        seeds = ["EARNED".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump,
        token::mint = mint,
        token::authority = earned_usdc_account,
    )]
    pub earned_usdc_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        seeds = ["RESERVE".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump,
        token::mint = mint,
        token::authority = reserve_usdc_account,
    )]
    pub reserve_usdc_account: Account<'info, TokenAccount>,

    // "USDC" Mint
    #[account(
        address = USDC_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account()]
    pub token_data: Box<Account<'info, TokenData>>,
    
    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), token_data.key().as_ref()],
        bump = token_data.mint_bump
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    // USDC to here
    #[account(
        mut,
        constraint = reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.reserve_bump,
    )]
    pub reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = treasury_account.mint == mint.key(),
    )]
    pub treasury_account: Box<Account<'info, TokenAccount>>,

    // Mint Tokens here
    #[account(mut,
        constraint = user_token.mint == token_mint.key(),
        constraint = user_token.owner == user.key() 
    )]
    pub user_token: Box<Account<'info, TokenAccount>>,

    // USDC from here
    #[account(mut,
        constraint = user_usdc_token.mint == mint.key(),
        constraint = user_usdc_token.owner == user.key()
    )]
    pub user_usdc_token: Box<Account<'info, TokenAccount>>,
    
    pub user: Signer<'info>,

    // "USDC" Mint
    #[account(
        address = USDC_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account()]
    pub token_data: Box<Account<'info, TokenData>>,

    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), token_data.key().as_ref()],
        bump = token_data.mint_bump
    )]
    pub token_mint: Box<Account<'info, Mint>>,
    

    // Mint Tokens here
    #[account(mut,
        constraint = user_token.mint == token_mint.key(),
        constraint = user_token.owner == user.key() 
    )]
    pub user_token: Box<Account<'info, TokenAccount>>,

    // The authority allowed to mutate the above ⬆
    pub user: Signer<'info>,

    // USDC to here
    #[account(
        mut,
        constraint = reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.reserve_bump,
    )]
    pub reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    // USDC to here
    #[account(
        mut,
        constraint = reserve_usdc_account.mint == mint.key(),
        seeds = ["EARNED".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.earned_bump,
    )]
    pub earned_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = treasury_account.mint == mint.key(),
    )]
    pub treasury_account: Box<Account<'info, TokenAccount>>,

    // "USDC" Mint
    #[account(
        address = USDC_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub mint: Account<'info, Mint>,

    // SPL Token Program
    pub token_program: Program<'info, Token>,

}


#[derive(Accounts)]
pub struct PartialRedeem<'info> {
    #[account()]
    pub token_data: Box<Account<'info, TokenData>>,

    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), token_data.key().as_ref()],
        bump = token_data.mint_bump
    )]
    pub token_mint: Box<Account<'info, Mint>>,
    

    // Mint Tokens here
    #[account(mut,
        constraint = user_token.mint == token_mint.key(),
        constraint = user_token.owner == user.key() 
    )]
    pub user_token: Box<Account<'info, TokenAccount>>,

    // USDC from here
    #[account(mut,
        constraint = user_usdc_token.mint == mint.key(),
        constraint = user_usdc_token.owner == user.key()
    )]
    pub user_usdc_token: Box<Account<'info, TokenAccount>>,

    // The authority allowed to mutate the above ⬆
    pub user: Signer<'info>,

    // USDC to here
    #[account(
        mut,
        constraint = reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.reserve_bump,
    )]
    pub reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    // USDC to here
    #[account(
        mut,
        constraint = reserve_usdc_account.mint == mint.key(),
        seeds = ["EARNED".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.earned_bump,
    )]
    pub earned_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = treasury_account.mint == mint.key(),
    )]
    pub treasury_account: Box<Account<'info, TokenAccount>>,

    // "USDC" Mint
    #[account(
        address = USDC_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub mint: Account<'info, Mint>,

    // SPL Token Program
    pub token_program: Program<'info, Token>,

}

#[account]
pub struct TokenData {
    pub name: String,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub earned: Pubkey,
    pub reserve: Pubkey,
    pub mint_bump: u8,
    pub earned_bump: u8,
    pub reserve_bump: u8,
    pub transaction_fee: u64,
    pub sale_fee: u64,
    pub discount: u64,
    pub reward: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("PDA not match")]
    PDA
}