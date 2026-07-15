use anchor_lang::prelude::*;

declare_id!("4XeRRycAsykrjrYYVwZLqtC4FCurZzwxnwPU2qonv3Ui");

#[program]
pub mod day2_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
