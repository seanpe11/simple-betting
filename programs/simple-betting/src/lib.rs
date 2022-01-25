use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use std::convert::{Into, From, TryFrom};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod simple_betting {
    use super::*;

    // processors
    pub fn intialize_betting(
        ctx: Context<InitializeBetting>, 
        authority: Pubkey,
        house_edge: u8
    ) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.round_id = 0;
        betting_round.authority = authority;
        betting_round.bull_bets = 0;
        betting_round.bear_bets = 0;
        betting_round.house_edge = house_edge;
        betting_round.bull_win = true;
        betting_round.finalized = false;

        // initialize vault to keep balances

        Ok(())
    }
    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, bull_bet: bool) -> ProgramResult {
        let bet_account = &mut ctx.accounts.placed_bet;
        bet_account.bettor = ctx.accounts.bettor.key();
        bet_account.bet = amount;
        bet_account.bet_bull = bull_bet;
        bet_account.betting_round = ctx.accounts.betting_round.key();

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
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info().clone(),
            to: ctx.accounts.edge_token_account.to_account_info().clone(),
            authority: ctx.accounts.vault.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
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
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info().clone(),
            to: ctx.accounts.bettor_token_account.to_account_info().clone(),
            authority: ctx.accounts.vault.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, token_payout)?;

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
    #[account(mut)]
    pub betting_round: Account<'info, BettingRound>,
    #[account(mut)]
    pub bettor_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub bettor: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
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
    #[account(mut)]
    pub bettor_token_account: Account<'info, TokenAccount>,
    pub bettor: Signer<'info>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub vault: AccountInfo<'info>,
    pub betting_round: Account<'info, BettingRound>,
    pub token_program: AccountInfo<'info>,
}

// accounts
#[account]
pub struct BettingRound {
    pub authority: Pubkey,
    pub round_id: u64,
    pub bull_win: bool,
    pub finalized: bool,
    pub bull_bets: u64,
    pub bear_bets: u64,
    pub house_edge: u8,
    pub vault: Pubkey,
}
impl BettingRound {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1 + 1 + 8 + 8 + 8 + 32;
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
