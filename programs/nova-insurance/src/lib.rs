use anchor_lang::prelude::*;

declare_id!("DB1ZyxKho5hwQPd6r7C1FSTifw5N7G5YYh5gyhvcpGN5");

pub mod errors;
pub mod state;

#[allow(unused_imports)]
use errors::*;
#[allow(unused_imports)]
use state::*;

#[program]
pub mod nova_insurance {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
