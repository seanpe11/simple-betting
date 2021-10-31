use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod simple_betting {
    use super::*;
    // processors
    pub fn initializeBetting(ctx: Context<InitializeBetting>) -> ProgramResult {
        Ok(())
    }
    pub fn placeBet(
        ctx: Context<PlaceBet>,
        amount: u64,
    ) -> ProgramResult {

    }
    pub fn claimBet(
        ctx: Context<ClaimBet>
    ) -> ProgramResult {

    }
    pub fn endRound(
        ctx: Context<EndRound>
    ) -> ProgramResult {

    }
}

// instructions
#[derive(Accounts)]
pub struct InitializeBetting {

}
#[derive(Accounts)]
pub struct PlaceBet {

}
#[derive(Accounts)]
pub struct ClaimBet {

}
#[derive(Accounts)]
pub struct EndRound {

}

// accounts
#[account]
pub struct BettingRound{
    pub authority: Pubkey,
    pub bull_bets: u64,
    pub bear_bets: u64,
    pub bull_win: bool
}
#[account]
pub struct BetAccount {
    pub bettor: Pubkey, // wallet who made the bet
    pub bet: u64, // amount bet
    pub bet_bull: bool, // if account bet on bull
    pub betting_round: Account<'info, BettingRound> // betting round account is associated with
}