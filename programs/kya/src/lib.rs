use anchor_lang::prelude::*;

declare_id!("BJNHHcf7Mpah8zxRj1375AfMAicuUd3rCBat7BCYEo4u");

const MAX_AGENT_NAME_LEN: usize = 32;
const MAX_REASONING_LEN: usize = 200;

#[program]
pub mod kya {
    use super::*;

    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        agent_name: String,
        max_amount: u64,
        logger_authority: Pubkey,
    ) -> Result<()> {
        require!(agent_name.len() <= MAX_AGENT_NAME_LEN, KyaError::StringTooLong);

        let agent = &mut ctx.accounts.agent_record;
        agent.owner = ctx.accounts.owner.key();
        agent.logger_authority = logger_authority;
        agent.agent_name = agent_name;
        agent.max_amount = max_amount;
        agent.trust_level = 50; // Стартовое доверие
        agent.total_logs = 0;
        agent.is_active = true;
        agent.created_at = Clock::get()?.unix_timestamp;
        agent.last_updated = agent.created_at;
        agent.bump = ctx.bumps.agent_record;

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
        require!(reasoning.len() <= MAX_REASONING_LEN, KyaError::StringTooLong);

        let intent_record = &mut ctx.accounts.intent_record;
        intent_record.agent = agent.key();
        intent_record.intent_id = intent_id;
        intent_record.decision = decision;
        intent_record.reasoning = reasoning;
        intent_record.amount = amount;
        intent_record.destination = destination;
        intent_record.timestamp = Clock::get()?.unix_timestamp;

        agent.total_logs = agent.total_logs.checked_add(1).ok_or(KyaError::Overflow)?;
        
        if decision == 0 { //Good Action
            if agent.trust_level < 100 { agent.trust_level += 1; }
        } else if decision == 1 { //Risky Action
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
    pub agent_name: String,   // 4 + 32
    pub max_amount: u64,      // 8
    pub trust_level: u8,      // 1
    pub total_logs: u64,      // 8
    pub is_active: bool,      // 1
    pub created_at: i64,      // 8
    pub last_updated: i64,    // 8
    pub bump: u8,             // 1
}

#[account]
pub struct IntentRecord {
    pub agent: Pubkey,        // 32
    pub intent_id: u64,       // 8
    pub decision: u8,         // 1
    pub reasoning: String,    // 4 + 200
    pub amount: u64,          // 8
    pub destination: Pubkey,  // 32
    pub timestamp: i64,       // 8
}

#[derive(Accounts)]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = owner,
        // 8 (дискриминатор) + 32 + 32 + 36 + 8 + 1 + 8 + 1 + 8 + 8 + 1
        space = 8 + 150, 
        seeds = [b"agent", owner.key().as_ref()],
        bump
    )]
    pub agent_record: Account<'info, AgentRecord>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(intent_id: u64)]
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
        init,
        payer = logger_authority,
        // 8 + 32 + 8 + 1 + (4 + 200) + 8 + 32 + 8
        space = 8 + 350,
        seeds = [b"intent", agent_record.key().as_ref(), intent_id.to_le_bytes().as_ref()],
        bump
    )]
    pub intent_record: Account<'info, IntentRecord>,

    /// CHECK: Проверяется через seeds в agent_record
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub logger_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
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
    #[msg("String too long")]
    StringTooLong,
    #[msg("Agent is not active")]
    AgentInactive,
    #[msg("Duplicate intent ID")]
    DuplicateIntent,
    #[msg("Calculation overflow")]
    Overflow,
}
