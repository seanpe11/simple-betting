use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod simple_betting {
    use super::*;
    // processors
    pub fn initializeBetting(
        ctx: Context<InitializeBetting>,
        authority: Pubkey
    ) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.authority = authority;
        betting_round.bull_bets = 0;
        betting_round.bear_bets = 0;
        betting_round.total_bets = 0;
        betting_round.bull_win = true;
        betting_round.finalized = false;
        Ok(())
    }
    pub fn placeBet(
        ctx: Context<PlaceBet>,
        amount: u64,
        bull_bet: bool,
        bettor: Pubkey,
        betting_round_param: Pubkey,
    ) -> ProgramResult {
        let bet_account = &mut ctx.accounts.bet_account;
        bet_account.bettor = bettor; // figure out how to extract pubkey from ctx
        bet_account.bet = amount;
        bet_account.bet_bull = bull_bet;
        bet_account.betting_round = betting_round_param;  // figure out how to extract pubkey from ctx

        let betting_round = &mut ctx.accounts.betting_round;
        if (bull_bet) {
            betting_round.bull_bets += amount;
        } else {
            betting_round.bear_bets += amount;
        }
        betting_round.total_bets += amount;
        Ok(())
    }
    pub fn endBet(
        ctx: Context<EndBet>,
        bull_win: bool,
    ) -> ProgramResult {
        let betting_round = &mut ctx.accounts.betting_round;
        betting_round.bull_win = bull_win;
        betting_round.finalized = true;
        Ok(())
    }
    pub fn claimBet(
        ctx: Context<ClaimBet>,
    ) -> ProgramResult {
        const bet_account = ctx.accounts.bet_account;
        const betting_round = ctx.accounts.bet_account;
        let mut to_withdraw: u64 = bet_account.bet;
        
        if bet_account.bet_bull && betting_round.bull_win {
            to_withdraw *= (betting_round.total_bets / betting_round.bull_bet);
        }
        else if !bet_account.bet_bull && !betting_round.bull_win {
            to_withdraw *= (betting_round.total_bets / betting_round.bear_bet);
        }

        // figure out how to send tokens after 

    }
}

// instructions
#[derive(Accounts)]
pub struct InitializeBetting {
    #[account(init, mut, payer = authority)]
    pub betting_round: Account<'info, BettingRound>,
    #[account(mut)]
    pub authority: Singer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct PlaceBet {
    #[account(mut)]
    pub betting_round: Account<'info, BettingRound>,
    #[account(init, mut, payer = bettor)]
    pub placed_bet: Account<'info, BetAccount>,
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct EndBet {
    #[account(mut, has_one = authority)]
    pub betting_round: Account<'info, BettingRound>,
    pub authority: Signer<'info>,
}
#[derive(Accounts)]
pub struct ClaimBet {
    #[account(mut, has_one = bettor)]
    pub bet_account: Account<'info, BetAccount>,
    pub bettor: Signer<'info>,
    #[account(mut, constraint = (finalized == true) )]
    pub betting_round: Account<'info, BettingRound>,
}

// accounts
#[account]
pub struct BettingRound{
    pub authority: Pubkey,
    pub bull_bets: u64,
    pub bear_bets: u64,
    pub total_bets: u64,
    pub bull_win: bool,
    pub finalized: bool,
}
#[account]
pub struct BetAccount {
    pub bettor: Pubkey, // wallet who made the bet
    pub bet: u64, // amount bet
    pub bet_bull: bool, // if account bet on bull
    pub claimed: bool,
    pub betting_round: Account<'info, BettingRound>,
}