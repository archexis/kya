use anchor_lang::prelude::*;

declare_id!("3QvkEo1ZxF9X39dvSDDi9Nx4ccMfbhyp1YhDY1apUHGg");

const MAX_LOGS: usize = 50;
const MAX_DECISION_LEN: usize = 64;

#[program]
pub mod kya {
    use super::*;

    pub fn register_agent(ctx: Context<RegisterAgent>) -> Result<()> {
        let agent = &mut ctx.accounts.agent_record;
        agent.owner = ctx.accounts.owner.key();
        agent.trust_level = 0;
        agent.bump = ctx.bumps.agent_record;
        agent.total_logs = 0;
        agent.version = 1;
        Ok(())
    }

    pub fn log_intent(
        ctx: Context<LogIntent>,
        intent_id: u64,
        decision: String,
        is_approved: bool,
    ) -> Result<()> {
        require!(decision.len() <= MAX_DECISION_LEN, KyaError::DecisionTooLong);

        let agent = &mut ctx.accounts.agent_record;

        // update trust
        if is_approved {
            agent.trust_level = agent.trust_level.saturating_add(1);
        } else {
            agent.trust_level = agent.trust_level.saturating_sub(2);
        }

        let log_acc = &mut ctx.accounts.intent_log;

        // ограничение
        require!(
            log_acc.logs.len() < MAX_LOGS,
            KyaError::LogOverflow
        );

        log_acc.logs.push(IntentItem {
            intent_id,
            decision,
            is_approved,
            timestamp: Clock::get()?.unix_timestamp,
        });

        agent.total_logs += 1;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = owner,
        space = AgentRecord::SIZE,
        seeds = [b"agent", owner.key().as_ref()],
        bump
    )]
    pub agent_record: Account<'info, AgentRecord>,

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
        has_one = owner
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(
        mut,
        seeds = [b"log", agent_record.key().as_ref()],
        bump
    )]
    pub intent_log: Account<'info, IntentLog>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

#[account]
pub struct AgentRecord {
    pub owner: Pubkey,
    pub trust_level: i64,
    pub total_logs: u64,
    pub version: u8,
    pub bump: u8,
}

impl AgentRecord {
    pub const SIZE: usize = 8 + 32 + 8 + 8 + 1 + 1;
}

#[account]
pub struct IntentLog {
    pub logs: Vec<IntentItem>,
}

impl IntentLog {
    pub const SIZE: usize =
        8 + // discriminator
        4 + // vec len
        (MAX_LOGS * IntentItem::SIZE);
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct IntentItem {
    pub intent_id: u64,
    pub decision: String,
    pub is_approved: bool,
    pub timestamp: i64,
}

impl IntentItem {
    pub const SIZE: usize =
        8 + // id
        4 + MAX_DECISION_LEN + // string
        1 + // bool
        8; // timestamp
}

#[error_code]
pub enum KyaError {
    #[msg("Decision too long")]
    DecisionTooLong,
    #[msg("Too many logs")]
    LogOverflow,
}
