use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use std::convert::{Into, From, TryFrom};
use spl_token::instruction::AuthorityType;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod simple_betting {
    use super::*;

    const SOLPREDICT_PDA_SEED: &[u8] = b"solpredict";

    // processors
    pub fn intialize_betting(
        ctx: Context<InitializeBetting>, 
        house_edge: u8
    ) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.round_id = 0;
        betting_round.authority = *ctx.accounts.authority.to_account_info().key;
        betting_round.bull_bets = 0;
        betting_round.bear_bets = 0;
        betting_round.house_edge = house_edge;
        betting_round.bull_win = true;
        betting_round.cancelled = false;
        betting_round.finalized = false;

        Ok(())
    }
    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, bull_bet: bool) -> ProgramResult {
        let bet_account = &mut ctx.accounts.placed_bet;
        bet_account.bettor = *ctx.accounts.bettor.to_account_info().key;
        bet_account.bet = amount;
        bet_account.bet_bull = bull_bet;
        bet_account.betting_round = *ctx.accounts.betting_round.to_account_info().key;

        let betting_round = &mut ctx.accounts.betting_round;
        if bull_bet {
            betting_round.bull_bets += amount;
        } else {
            betting_round.bear_bets += amount;
        };

        // TODO: transfer tokens from bettor to PDA

        let cpi_accounts = Transfer {
            from: ctx.accounts.bettor_token_account.to_account_info().clone(),
            to: ctx.accounts.vault_token_account.to_account_info().clone(),
            authority: ctx.accounts.bettor.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn end_betting(ctx: Context<EndBetting>, bull_win: bool) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.bull_win = bull_win;
        betting_round.finalized = true;
        // deduct edge from data
        betting_round.bull_bets = u64::try_from(
                u128::from(betting_round.bull_bets) * u128::from(100 - betting_round.house_edge) / 100 
            ).unwrap();
        betting_round.bear_bets = u64::try_from(
                u128::from(betting_round.bear_bets) * u128::from(100 - betting_round.house_edge) / 100 
            ).unwrap();
        let total_bets = u128::from(betting_round.bear_bets + betting_round.bull_bets);
        let edge: u64 = u64::try_from(
                total_bets * u128::from(betting_round.house_edge) / 100 
            ).unwrap();
        
        // payout to edge account
        let (_pda, bump_seed) = Pubkey::find_program_address(&[SOLPREDICT_PDA_SEED], ctx.program_id);
        let seeds = &[&SOLPREDICT_PDA_SEED[..], &[bump_seed]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info().clone(),
            to: ctx.accounts.edge_token_account.to_account_info().clone(),
            authority: ctx.accounts.vault.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, edge)?;

        msg!(
            "Finalized round {} with outcome {}, {}SOL bull, {}SOL bear.",
            betting_round.round_id,
            betting_round.bull_win,
            betting_round.bull_bets,
            betting_round.bear_bets,
        );
        Ok(())
    }
    pub fn claim_bet(ctx: Context<ClaimBet>) -> ProgramResult {
        let bet_account = &mut ctx.accounts.bet_account;
        let betting_round = &mut ctx.accounts.betting_round;
        
        // calculate necessary stuff for payout after edge
        let total_bets = u128::from(betting_round.bear_bets + betting_round.bull_bets);
        let winning_share = if betting_round.bull_win {
            u128::from(betting_round.bull_bets)
        } else {
            u128::from(betting_round.bear_bets)
        };
        let bet_after_edge = u128::from(bet_account.bet) * u128::from(betting_round.house_edge) / 100; 

        let token_payout = u64::try_from(
            total_bets *  bet_after_edge / winning_share
        ).unwrap();

        msg!("Paid out {} to winning bettor", token_payout);

        // TODO: transfer tokens from PDA to bettor
        let (_pda, bump_seed) = Pubkey::find_program_address(&[SOLPREDICT_PDA_SEED], ctx.program_id);
        let seeds = &[&SOLPREDICT_PDA_SEED[..], &[bump_seed]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info().clone(),
            to: ctx.accounts.bettor_token_account.to_account_info().clone(),
            authority: ctx.accounts.vault.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, token_payout)?;

        Ok(())
    }
    pub fn cancel_round(ctx: Context<CancelRound>) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.cancelled = true;

        Ok(())
    }
    pub fn claim_cancelled(ctx: Context<ClaimCancelled>) -> ProgramResult {

        // TODO: transfer tokens from PDA to bettor
        let (_pda, bump_seed) = Pubkey::find_program_address(&[SOLPREDICT_PDA_SEED], ctx.program_id);
        let seeds = &[&SOLPREDICT_PDA_SEED[..], &[bump_seed]];
        let signer = &[&seeds[..]];
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info().clone(),
            to: ctx.accounts.bettor_token_account.to_account_info().clone(),
            authority: ctx.accounts.vault.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.bet_account.bet)?;

        Ok(())
    }
}

// instructions
#[derive(Accounts)]
pub struct InitializeBetting<'info> {
    #[account(init, payer = authority, space = 8 + BettingRound::LEN)]
    pub betting_round: Account<'info, BettingRound>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(init, payer = bettor, space = 8 + BetAccount::LEN)]
    pub placed_bet: Account<'info, BetAccount>,
    #[account(
        mut,
        constraint = betting_round.finalized == false,
        constraint = betting_round.cancelled == false
    )]
    pub betting_round: Account<'info, BettingRound>,
     #[account(
        mut,
        constraint = bettor_token_account.to_account_info().owner == bettor.to_account_info().key
    )]
    pub bettor_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub bettor: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        mut,
        constraint = vault_token_account.to_account_info().owner == vault.to_account_info().key
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}
#[derive(Accounts)]
pub struct EndBetting<'info> {
    #[account(mut, has_one = authority)]
    pub betting_round: Account<'info, BettingRound>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub edge_token_account: Account<'info, TokenAccount>,
    pub token_program: AccountInfo<'info>,
}
#[derive(Accounts)]
pub struct ClaimBet<'info> {
    #[account(mut, has_one = bettor, 
        constraint = bet_account.bet_bull == betting_round.bull_win,
        constraint = betting_round.finalized == true
    )]
    pub bet_account: Account<'info, BetAccount>,
     #[account(
        mut,
        constraint = bettor_token_account.to_account_info().owner == bettor.to_account_info().key
    )]
    pub bettor_token_account: Account<'info, TokenAccount>,
    pub bettor: Signer<'info>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    pub betting_round: Account<'info, BettingRound>,
    pub token_program: AccountInfo<'info>,
}
#[derive(Accounts)]
pub struct ClaimCancelled<'info> {
    #[account(mut, has_one = bettor, 
        constraint = bet_account.bet_bull == betting_round.bull_win,
        constraint = betting_round.cancelled == true
    )]
    pub bet_account: Account<'info, BetAccount>,
     #[account(
        mut,
        constraint = bettor_token_account.to_account_info().owner == bettor.to_account_info().key
    )]
    pub bettor_token_account: Account<'info, TokenAccount>,
    pub bettor: Signer<'info>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    pub betting_round: Account<'info, BettingRound>,
    pub token_program: AccountInfo<'info>,
}
#[derive(Accounts)]
pub struct CancelRound<'info> {
    #[account(mut, has_one = authority)]
    pub betting_round: Account<'info, BettingRound>,
    pub authority: Signer<'info>,
}

// accounts
#[account]
pub struct BettingRound {
    pub round_id: u64,
    pub bull_win: bool,
    pub finalized: bool,
    pub cancelled: bool,
    pub bull_bets: u64,
    pub bear_bets: u64,
    pub house_edge: u8,
    pub authority: Pubkey,
    pub vault: Pubkey,
}
impl BettingRound {
    pub const LEN: usize = 8 + 1 + 1 + 1 + 8 + 8 + 1 + 32 + 32;
}
#[account]
pub struct BetAccount {
    pub bettor: Pubkey, // wallet who made the bet
    pub bet: u64,       // amount bet
    pub bet_bull: bool, // if account bet on bull
    pub claimed: bool,
    pub betting_round: Pubkey, // associated betting account
}
impl BetAccount {
    pub const LEN: usize = 32 + 8 + 1 + 1 + 32;
}
