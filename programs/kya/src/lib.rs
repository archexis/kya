use anchor_lang::prelude::*;

declare_id!("3HtY7pQE8pcsjJ2BxhadeSg7ZoU7Hm28pgMRYCpj5n4g");

#[program]
pub mod kya {
    use super::*;

    pub fn register_agent(ctx: Context<RegisterAgent>) -> Result<()> {
        let agent_record = &mut ctx.accounts.agent_record;
        agent_record.owner = *ctx.accounts.owner.key;
        agent_record.trust_level = 0;
        agent_record.total_logs = 0;
        agent_record.bump = ctx.bumps.agent_record;
        Ok(())
    }

    pub fn log_intent(
        ctx: Context<LogIntent>,
        intent_id: u64,
        decision: String,
        is_approved: bool,
    ) -> Result<()> {
        let agent_record = &mut ctx.accounts.agent_record;
        let intent_log = &mut ctx.accounts.intent_log;

        // Создаем запись лога
        let entry = IntentEntry {
            intent_id,
            decision,
            is_approved,
            timestamp: Clock::get()?.unix_timestamp,
        };

        if intent_log.logs.len() >= 10 {
            intent_log.logs.remove(0);
        }
        intent_log.logs.push(entry);

        agent_record.total_logs += 1;
        if is_approved {
            agent_record.trust_level += 1;
        }

        Ok(())
    }
}

#[account]
pub struct AgentRecord {
    pub owner: Pubkey,      // 32
    pub trust_level: u64,   // 8
    pub total_logs: u64,    // 8
    pub bump: u8,           // 1
}

#[account]
pub struct IntentLog {
    pub logs: Vec<IntentEntry>, // Макс 10 записей для экономии места
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct IntentEntry {
    pub intent_id: u64,
    pub decision: String,
    pub is_approved: bool,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 8 + 8 + 1,
        seeds = [b"agent", owner.key().as_ref()],
        bump
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(
        init,
        payer = owner,
        space = 8 + (4 + (10 * 100)), // Примерно 1кб под логи
        seeds = [b"log", owner.key().as_ref()],
        bump
    )]
    pub intent_log: Account<'info, IntentLog>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LogIntent<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_record.bump,
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(
        mut,
        seeds = [b"log", owner.key().as_ref()],
        bump,
    )]
    pub intent_log: Account<'info, IntentLog>,

    #[account(mut)]
    pub owner: Signer<'info>,
}
