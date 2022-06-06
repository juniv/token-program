use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount,};

declare_id!("G28ceN5471mPMKhSThZu4tvzK6Skbxrr8qy4abskVsYJ");

// Replace for Devnet Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr
// Replace for Localnet 8fFnX9WSPjJEADtG5jQvQQptzfFmmjd6hrW7HjuUT8ur
pub const USDC_MINT_ADDRESS: &str = "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";

pub const AUTHORITY: &str = "DfLZV18rD7wCQwjYvhTFwuvLh49WSbXFeJFPQb5czifH";


#[program]
pub mod token3 {
    use super::*;

    // create program treasury USDC account
    pub fn init_treasury(_ctx: Context<InitTreasury>) -> Result<()> {
       Ok(())
    }
    
    // create new token_data account 
    // create new reward token mint
    // create reserve and earned USDC acounts for new reward token
    // store fields in token_data account
    pub fn new_token(ctx: Context<NewToken>, name: String, transaction_fee: u64, sale_fee: u64, discount: u64, reward_generic_token: u64, reward_merchant_token: u64, reward_usdc_token: u64) -> Result<()> {
        //Derive PDAs for Mint, Earned, Reserve
        let (token_pda, token_bump) =
            Pubkey::find_program_address(&["MINT".as_bytes(), ctx.accounts.token_data.key().as_ref()], ctx.program_id);

        let (earned_pda, earned_bump) =
            Pubkey::find_program_address(&["EARNED".as_bytes(), ctx.accounts.token_data.key().as_ref(), ctx.accounts.mint.key().as_ref()], ctx.program_id);
        
        let (reserve_pda, reserve_bump) =
            Pubkey::find_program_address(&["RESERVE".as_bytes(), ctx.accounts.token_data.key().as_ref(), ctx.accounts.mint.key().as_ref()], ctx.program_id);

        // check derived PDA matches account provided
        if token_pda != ctx.accounts.token_mint.key() {
            return err!(ErrorCode::PDA);
        }

        if earned_pda != ctx.accounts.earned_usdc_account.key() {
            return err!(ErrorCode::PDA);
        }

        if reserve_pda != ctx.accounts.reserve_usdc_account.key() {
            return err!(ErrorCode::PDA);
        }

        // update fields on token_data account
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
        token_data.reward_generic_token = reward_generic_token;
        token_data.reward_merchant_token = reward_merchant_token;
        token_data.reward_usdc_token = reward_usdc_token;
        
        Ok(())
    }

    // mint reward tokens in exchange for "USDC"
    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        // derive treasury_pda
        let (treasury_pda, _treasury_bump) =
            Pubkey::find_program_address(&["TREASURY".as_bytes(), ctx.accounts.mint.key().as_ref()], ctx.program_id);
        
        // check derived PDA matches account provided
        if treasury_pda != ctx.accounts.treasury_account.key() {
            return err!(ErrorCode::PDA);
        }

        let token_data = ctx.accounts.token_data.key();

        // mint tokens to user
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
        
        // TODO: Account for Decimals in Mint (right now both tokens same decimals for simplicity)
        // TODO: Safe Math
        let usdc_amount = amount * (10000-discount) / 10000;
        let fee_amount = usdc_amount * (sale_fee) / 10000;
        let reserve_amount = usdc_amount - fee_amount ;
        
        // transfer USDC from the User to Treasury
        // fee amount
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
        // amount of USDC backing minted tokens
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

    // redeem using only USDC
    pub fn redeem_usdc(ctx: Context<RedeemUsdc>, amount: u64,) -> Result<()> {
        // derive treasury_pda
        let (treasury_pda, _treasury_bump) =
            Pubkey::find_program_address(&["TREASURY".as_bytes(), ctx.accounts.mint.key().as_ref()], ctx.program_id);
        
        // check derived PDA matches account provided
        if treasury_pda != ctx.accounts.treasury_account.key() {
            return err!(ErrorCode::PDA);
        }

        let token_data = ctx.accounts.token_data.key();
        
        // transaction fee
        let fee_amount = ctx.accounts.token_data.transaction_fee; 
        // rebate merchant token amount (amount to mint)
        let reward_amount = amount * ctx.accounts.token_data.reward_usdc_token / 10000;
        // amount USDC sent to merchant earned account
        let earned_amount = amount - reward_amount - fee_amount;
        
        // mint reward token to user
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

        token::mint_to(cpi_ctx, reward_amount)?;
    
        // transfer USDC fee from User to treasury
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC from user to reserve
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.reserve_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, reward_amount)?;

        // transfer USDC earned
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, earned_amount)?;
        
        Ok(())
    }

    // redeem only using one reward token
    pub fn redeem_one_token(ctx: Context<RedeemOneToken>, amount: u64,) -> Result<()> {
        let token_data = ctx.accounts.token_data.key();
        let reward_amount = amount * ctx.accounts.token_data.reward_merchant_token / 10000;
        let fee_amount = ctx.accounts.token_data.transaction_fee; 
        let usdc_value = (amount - reward_amount) * (ctx.accounts.reserve_usdc_account.amount) / (ctx.accounts.token_mint.supply);
        let earned_amount = usdc_value - fee_amount;
        
        // burn tokens        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.token_mint.to_account_info(),
                from: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)?;
        
        // mint is usdc mint
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), token_data.as_ref(), mint.as_ref(), &[ctx.accounts.token_data.reserve_bump]];
        let signer = [&seeds[..]];

        // transfer USDC fee from reserve to treasury
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
            &signer,
        );

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
        
        token::transfer(cpi_ctx, earned_amount)?;

        // mint reward token to user
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

        token::mint_to(cpi_ctx, reward_amount)?;
        
        
        Ok(())
    }

    // redeem only using one reward token
    pub fn redeem_one_generic_token(ctx: Context<RedeemOneGenericToken>, amount: u64,) -> Result<()> {
        let generic_token_data = ctx.accounts.generic_token_data.key();
        let token_data = ctx.accounts.token_data.key();
        let reward_amount = amount * ctx.accounts.generic_token_data.reward_merchant_token / 10000;
        let fee_amount = ctx.accounts.generic_token_data.transaction_fee; 
        let usdc_value = (amount - reward_amount) * (ctx.accounts.generic_reserve_usdc_account.amount) / (ctx.accounts.generic_token_mint.supply);
        let earned_amount = usdc_value - fee_amount;
        
        // burn tokens        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.generic_token_mint.to_account_info(),
                from: ctx.accounts.user_generic_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)?;
        
        // mint is usdc mint
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), generic_token_data.as_ref(), mint.as_ref(), &[ctx.accounts.generic_token_data.reserve_bump]];
        let signer = [&seeds[..]];

        // transfer USDC fee from reserve to treasury
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
            &signer,
        );

        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC earned
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        token::transfer(cpi_ctx, earned_amount)?;

        // mint reward token to user
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

        token::mint_to(cpi_ctx, reward_amount)?;
        
        
        Ok(())
    }

    // redeem using two reward tokens
    pub fn redeem_two_token(ctx: Context<RedeemTwoToken>, token_amount: u64, usdc_amount:u64) -> Result<()> {
        let token_data = ctx.accounts.token_data.key();
        let token_reward_amount = token_amount * ctx.accounts.token_data.reward_merchant_token / 10000;
        let usdc_reward_amount = usdc_amount * ctx.accounts.token_data.reward_usdc_token / 10000;
        let total_reward_amount = token_reward_amount + usdc_reward_amount;
        let usdc_value = (token_amount - token_reward_amount) * (ctx.accounts.reserve_usdc_account.amount) / (ctx.accounts.token_mint.supply);
        let fee_amount = ctx.accounts.token_data.transaction_fee; 
        let earned_amount = usdc_value - fee_amount;
        let usdc_earned_amount = usdc_amount - usdc_reward_amount; 

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.token_mint.to_account_info(),
                from: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, token_amount)?;
        
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

        token::transfer(cpi_ctx, fee_amount)?;

        // transfer USDC from reserve to earned
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.reserve_usdc_account.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        token::transfer(cpi_ctx, earned_amount)?;

        // transfer USDC from the User to earned
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.earned_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, usdc_earned_amount)?;

        // transfer USDC from the User to reserve
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.reserve_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, usdc_reward_amount)?;

        // mint reward token to user
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

        token::mint_to(cpi_ctx, total_reward_amount)?;
        
        
        Ok(())
    }

    // TODO: Check math
    // redeem using two reward tokens and USDC
    pub fn redeem_three_token(ctx: Context<RedeemThreeToken>, merchant_token_amount: u64, generic_token_amount: u64, usdc_amount:u64) -> Result<()> {
        let merchant_token_data = ctx.accounts.merchant_token_data.key();
        let generic_token_data = ctx.accounts.generic_token_data.key();
        
        // reward for merchant token spent
        let merchant_token_reward_amount = merchant_token_amount * ctx.accounts.merchant_token_data.reward_merchant_token / 10000;
        // reward for generic token spent 
        let generic_token_reward_amount = generic_token_amount * ctx.accounts.merchant_token_data.reward_generic_token / 10000;
        // reward for usdc token spent
        let usdc_reward_amount = usdc_amount * ctx.accounts.merchant_token_data.reward_usdc_token / 10000;

        // total rewards minted to user
        let total_reward_amount = merchant_token_amount +  generic_token_reward_amount + usdc_reward_amount; 
        
        // usdc value of merchant tokens redeemed less rewards rebated
        let merchant_usdc_earned = (merchant_token_amount - merchant_token_reward_amount) * (ctx.accounts.merchant_reserve_usdc_account.amount) / (ctx.accounts.merchant_token_mint.supply);
        // usdc value of generic tokens redeemed less rewards rebated
        let generic_usdc_earned = (generic_token_amount - generic_token_reward_amount) * (ctx.accounts.generic_reserve_usdc_account.amount) / (ctx.accounts.generic_token_mint.supply);
        // usdc value of generic tokens rewards transfer to merchant reserve
        let generic_usdc_reward = (generic_token_reward_amount) * (ctx.accounts.generic_reserve_usdc_account.amount) / (ctx.accounts.generic_token_mint.supply);
        
        // fixed transaction fee
        let fee_amount = ctx.accounts.merchant_token_data.transaction_fee; 
        let usdc_earned_amount = usdc_amount - usdc_reward_amount - fee_amount; 

        // burn merchant tokens
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.merchant_token_mint.to_account_info(),
                from: ctx.accounts.user_merchant_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, merchant_token_amount)?;

        // burn generic tokens
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.generic_token_mint.to_account_info(),
                from: ctx.accounts.user_generic_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, generic_token_amount)?;
        
        // merchant PDA signing
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), merchant_token_data.as_ref(), mint.as_ref(), &[ctx.accounts.merchant_token_data.reserve_bump]];
        let signer = [&seeds[..]];
        
        // transfer USDC to merchant earned account.
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.merchant_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.merchant_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.merchant_earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        token::transfer(cpi_ctx, merchant_usdc_earned)?;

        // merchant PDA signing
        let mint = ctx.accounts.mint.key();
        let seeds = &["RESERVE".as_bytes(), generic_token_data.as_ref(), mint.as_ref(), &[ctx.accounts.generic_token_data.reserve_bump]];
        let signer = [&seeds[..]];
        
        // transfer USDC to merchant earned account.
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.merchant_earned_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        token::transfer(cpi_ctx, generic_usdc_earned)?;

        // transfer USDC to merchant reserve.
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.generic_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.merchant_reserve_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        token::transfer(cpi_ctx, generic_usdc_reward)?;

        // transfer USDC from the User to earned
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.merchant_earned_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, usdc_earned_amount)?;

        // transfer USDC from the User to reserve
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.merchant_reserve_usdc_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, usdc_reward_amount)?;

        
        // transfer USDC fee from user to treasury
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_usdc_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, fee_amount)?;

        // mint reward token to user
        let seeds = &["MINT".as_bytes(), merchant_token_data.as_ref(), &[ctx.accounts.merchant_token_data.mint_bump]];
        let signer = [&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.merchant_token_mint.to_account_info(),
                to: ctx.accounts.user_merchant_token.to_account_info(),
                authority: ctx.accounts.merchant_token_mint.to_account_info(),
            },
            &signer,
        );

        token::mint_to(cpi_ctx, total_reward_amount)?;
        
        
        Ok(())
    }

    // withdraw from USDC from earned accounts to program treasury
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        
        let token_data = ctx.accounts.token_data.key();
        let mint = ctx.accounts.mint.key();
        let seeds = &["EARNED".as_bytes(), token_data.as_ref(), mint.as_ref(), &[ctx.accounts.token_data.earned_bump]];
        let signer = [&seeds[..]];

        // transfer USDC earned
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.earned_usdc_account.to_account_info(),
                authority: ctx.accounts.earned_usdc_account.to_account_info(),
                to: ctx.accounts.withdraw_usdc_account.to_account_info(),
            },
            &signer,
        );
        
        let amount = ctx.accounts.earned_usdc_account.amount;
        token::transfer(cpi_ctx, amount)?;
        
        
        Ok(())
    }

    //TODO: does each field need own function to update? can inputs be conditional?
    // update token account data
    pub fn update_token_data(ctx: Context<UpdateTokenData>, name: String, discount: u64, reward_usdc_token: u64) -> Result<()> {
        
        let token_data = &mut ctx.accounts.token_data;
        
        token_data.name = name;
        token_data.discount = discount;
        token_data.reward_usdc_token = reward_usdc_token;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitTreasury<'info> {
    #[account(
        init,
        payer = user,
        seeds = ["TREASURY".as_bytes().as_ref(), mint.key().as_ref() ],
        bump,
        token::mint = mint,
        token::authority = treasury_usdc_account,
    )]
    pub treasury_usdc_account: Account<'info, TokenAccount>,

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
pub struct RedeemUsdc<'info> {
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
    
    
    // Transaction fee to here
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
pub struct RedeemOneToken<'info> {
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
pub struct RedeemOneGenericToken<'info> {
    #[account()]
    pub generic_token_data: Box<Account<'info, TokenData>>,

    #[account()]
    pub token_data: Box<Account<'info, TokenData>>,


    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), generic_token_data.key().as_ref()],
        bump = generic_token_data.mint_bump
    )]
    pub generic_token_mint: Box<Account<'info, Mint>>,

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

    // Mint Tokens here
    #[account(mut,
        constraint = user_generic_token.mint == generic_token_mint.key(),
        constraint = user_generic_token.owner == user.key() 
    )]
    pub user_generic_token: Box<Account<'info, TokenAccount>>,

    // The authority allowed to mutate the above ⬆
    pub user: Signer<'info>,

    // USDC to here
    #[account(
        mut,
        constraint = generic_reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), generic_token_data.key().as_ref(), mint.key().as_ref() ],
        bump = generic_token_data.reserve_bump,
    )]
    pub generic_reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    // USDC to here
    #[account(
        mut,
        constraint = earned_usdc_account.mint == mint.key(),
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
pub struct RedeemTwoToken<'info> {
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

#[derive(Accounts)]
pub struct RedeemThreeToken<'info> {
    #[account()]
    pub merchant_token_data: Box<Account<'info, TokenData>>,

    #[account()]
    pub generic_token_data: Box<Account<'info, TokenData>>,

    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), merchant_token_data.key().as_ref()],
        bump = merchant_token_data.mint_bump
    )]
    pub merchant_token_mint: Box<Account<'info, Mint>>,

    #[account(mut,
        seeds = ["MINT".as_bytes().as_ref(), generic_token_data.key().as_ref()],
        bump = generic_token_data.mint_bump
    )]
    pub generic_token_mint: Box<Account<'info, Mint>>,
    
    // burn and mint tokens here
    #[account(mut,
        constraint = user_merchant_token.mint == merchant_token_mint.key(),
        constraint = user_merchant_token.owner == user.key() 
    )]
    pub user_merchant_token: Box<Account<'info, TokenAccount>>,

    //burn generic tokens here
    #[account(mut,
        constraint = user_generic_token.mint == generic_token_mint.key(),
        constraint = user_generic_token.owner == user.key() 
    )]
    pub user_generic_token: Box<Account<'info, TokenAccount>>,

    // USDC from here
    #[account(mut,
        constraint = user_usdc_token.mint == mint.key(),
        constraint = user_usdc_token.owner == user.key()
    )]
    pub user_usdc_token: Box<Account<'info, TokenAccount>>,

    // The authority allowed to mutate the above ⬆
    pub user: Signer<'info>,

    // USDC from here
    #[account(
        mut,
        constraint = merchant_reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), merchant_token_data.key().as_ref(), mint.key().as_ref() ],
        bump = merchant_token_data.reserve_bump,
    )]
    pub merchant_reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    // USDC from here
    #[account(
        mut,
        constraint = generic_reserve_usdc_account.mint == mint.key(),
        seeds = ["RESERVE".as_bytes().as_ref(), generic_token_data.key().as_ref(), mint.key().as_ref() ],
        bump = generic_token_data.reserve_bump,
    )]
    pub generic_reserve_usdc_account: Box<Account<'info, TokenAccount>>,

    // USDC to here
    #[account(
        mut,
        constraint = merchant_reserve_usdc_account.mint == mint.key(),
        seeds = ["EARNED".as_bytes().as_ref(), merchant_token_data.key().as_ref(), mint.key().as_ref() ],
        bump = merchant_token_data.earned_bump,
    )]
    pub merchant_earned_usdc_account: Box<Account<'info, TokenAccount>>,

    //TODO: add constraint, specify treasury account address
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
pub struct Withdraw<'info> {
    #[account()]
    pub token_data: Box<Account<'info, TokenData>>,

    // USDC to here
    #[account(
        mut,
        constraint = earned_usdc_account.mint == mint.key(),
        seeds = ["EARNED".as_bytes().as_ref(), token_data.key().as_ref(), mint.key().as_ref() ],
        bump = token_data.earned_bump,
    )]
    pub earned_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint =withdraw_usdc_account.mint == mint.key(),
    )]
    pub withdraw_usdc_account: Box<Account<'info, TokenAccount>>,

    // "USDC" Mint
    #[account(
        address = USDC_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub mint: Account<'info, Mint>,

    // SPL Token Program
    pub token_program: Program<'info, Token>,

    // Require Authority Signiture to Withdraw
    #[account(
        address = AUTHORITY.parse::<Pubkey>().unwrap(),
    )]
    pub authority: Signer<'info>,

}

#[derive(Accounts)]
pub struct UpdateTokenData<'info> {
    #[account(mut)]
    pub token_data: Box<Account<'info, TokenData>>,

    // Require Authority Signiture to Withdraw
    #[account(
        constraint = token_data.user == user.key()
    )]
    pub user: Signer<'info>,
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
    pub transaction_fee: u64, // fee per transaction
    pub sale_fee: u64, // usdc -> diam fee
    pub discount: u64, // usdc -> merchant token discount
    pub reward_generic_token: u64, // token -> mint on redemption
    pub reward_merchant_token: u64, // token -> mint on redemption
    pub reward_usdc_token: u64, // token -> mint on redemption
}

#[error_code]
pub enum ErrorCode {
    #[msg("PDA not match")]
    PDA
}