use anchor_lang::prelude::*;

declare_id!("DB1ZyxKho5hwQPd6r7C1FSTifw5N7G5YYh5gyhvcpGN5");

#[program]
pub mod nova_insurance {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
