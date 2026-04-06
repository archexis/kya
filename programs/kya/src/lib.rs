use anchor_lang::prelude::*;

declare_id!("BJNHHcf7Mpah8zxRj1375AfMAicuUd3rCBat7BCYEo4u");

const MAX_AGENT_NAME_BYTES: usize = 32;
const MAX_REASONING_BYTES: usize = 200;
const MAX_LOGS: usize = 10;

const INTENT_ENTRY_SPACE: usize = 8 + 1 + (4 + MAX_REASONING_BYTES) + 8 + 32 + 8;

#[program]
pub mod kya {
    use super::*;

    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        agent_name: String,
        max_amount: u64,
        logger_authority: Pubkey,
    ) -> Result<()> {
        require!(agent_name.len() <= MAX_AGENT_NAME_BYTES, KyaError::StringTooLong);

        let agent = &mut ctx.accounts.agent_record;
        let log = &mut ctx.accounts.intent_log;

        agent.owner = ctx.accounts.owner.key();
        agent.logger_authority = logger_authority;
        agent.agent_name = agent_name;
        agent.max_amount = max_amount;
        agent.trust_level = 50;
        agent.total_logs = 0;
        agent.is_active = true;
        agent.created_at = Clock::get()?.unix_timestamp;
        agent.last_updated = agent.created_at;
        agent.bump = ctx.bumps.agent_record;

        log.agent = agent.key();
        log.logs = Vec::new();
        log.bump = ctx.bumps.intent_log;

        Ok(())
    }

    pub fn log_intent(
        ctx: Context<LogIntent>,
        intent_id: u64,
        decision: u8,
        reasoning: String,
        amount: u64,
        destination: Pubkey,
    ) -> Result<()> {
        let agent = &mut ctx.accounts.agent_record;
        require!(agent.is_active, KyaError::AgentInactive);
        require!(reasoning.len() <= MAX_REASONING_BYTES, KyaError::StringTooLong);

        let intent_log = &mut ctx.accounts.intent_log;

        for entry in intent_log.logs.iter() {
            require!(entry.intent_id != intent_id, KyaError::DuplicateIntent);
        }

        let entry = IntentEntry {
            intent_id,
            decision,
            reasoning,
            amount,
            destination,
            timestamp: Clock::get()?.unix_timestamp,
        };

        if intent_log.logs.len() >= MAX_LOGS {
            intent_log.logs.remove(0);
        }
        intent_log.logs.push(entry);

        agent.total_logs = agent.total_logs.checked_add(1).ok_or(KyaError::Overflow)?;
        
        if decision == 0 {
            if agent.trust_level < 100 { agent.trust_level += 1; }
        } else if decision == 1 {
            agent.trust_level = agent.trust_level.saturating_sub(2);
        }

        agent.last_updated = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn deactivate_agent(ctx: Context<UpdateAgent>) -> Result<()> {
        let agent = &mut ctx.accounts.agent_record;
        agent.is_active = false;
        Ok(())
    }
}

#[account]
pub struct AgentRecord {
    pub owner: Pubkey,
    pub logger_authority: Pubkey,
    pub agent_name: String,
    pub max_amount: u64,
    pub trust_level: u8,
    pub total_logs: u64,
    pub is_active: bool,
    pub created_at: i64,
    pub last_updated: i64,
    pub bump: u8,
}

#[account]
pub struct IntentLog {
    pub agent: Pubkey,
    pub logs: Vec<IntentEntry>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct IntentEntry {
    pub intent_id: u64,
    pub decision: u8,
    pub reasoning: String,
    pub amount: u64,
    pub destination: Pubkey,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = owner,
        space = 250,
        seeds = [b"agent", owner.key().as_ref()],
        bump
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 4 + (MAX_LOGS * INTENT_ENTRY_SPACE) + 1,
        seeds = [b"log", agent_record.key().as_ref()],
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
        has_one = owner,
        has_one = logger_authority
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(
        mut,
        seeds = [b"log", agent_record.key().as_ref()],
        bump = intent_log.bump
    )]
    pub intent_log: Account<'info, IntentLog>,

    /// CHECK: safety check for pda seeds
    pub owner: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub logger_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAgent<'info> {
    #[account(
        mut,
        seeds = [b"agent", owner.key().as_ref()],
        bump = agent_record.bump,
        has_one = owner
    )]
    pub agent_record: Account<'info, AgentRecord>,
    pub owner: Signer<'info>,
}

#[error_code]
pub enum KyaError {
    #[msg("string too long")]
    StringTooLong,
    #[msg("agent is not active")]
    AgentInactive,
    #[msg("duplicate intent id")]
    DuplicateIntent,
    #[msg("calculation overflow")]
    Overflow,
}
