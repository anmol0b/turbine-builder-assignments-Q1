use crate::{errors::DiceError, Bet};
use anchor_instruction_sysvar::Ed25519InstructionSignatures;
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use solana_program::{
    ed25519_program,
    hash::hash,
    sysvar::instructions::{
        load_current_index_checked, load_instruction_at_checked, ID as InstructionSysvarId,
    },
};

#[constant]
const HOUSE_FEE_BPS: u64 = 150;
const BPS_DENOMINATOR: u64 = 10_000;

#[derive(Accounts)]
pub struct ResolveBet<'info> {
    #[account(mut)]
    pub house: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", house.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,

    /// CHECK: verified against bet.player and ed25519 signature
    #[account(mut)]
    pub player: UncheckedAccount<'info>,

    #[account(
        mut,
        close = player,
        has_one = player,
        seeds = [b"bet", vault.key().as_ref(), bet.seed.to_le_bytes().as_ref()],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,

    /// CHECK: instruction sysvar
    #[account(address = InstructionSysvarId)]
    pub instructions: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ResolveBet<'info> {
    pub fn verify_ed25519_signature(&self, sig: &[u8]) -> Result<()> {
        require_eq!(sig.len(), 64, DiceError::InvalidSignatureLength);

        let current_index = load_current_index_checked(&self.instructions)?;
        require!(current_index > 0, DiceError::MissingEd25519Instruction);

        let ed25519_ix =
            load_instruction_at_checked((current_index - 1) as usize, &self.instructions)?;

        require_keys_eq!(
            ed25519_ix.program_id,
            ed25519_program::ID,
            DiceError::Ed25519Program
        );

        require_eq!(
            ed25519_ix.accounts.len(),
            0,
            DiceError::Ed25519Accounts
        );

        let signatures = Ed25519InstructionSignatures::unpack(&ed25519_ix.data)
            .map_err(|_| DiceError::Ed25519DataLength)?
            .0;

        require_eq!(signatures.len(), 1, DiceError::Ed25519Signature);

        let signature = &signatures[0];

        require!(signature.is_verifiable, DiceError::Ed25519Header);

        let signer_pubkey = signature.public_key.ok_or(DiceError::Ed25519Pubkey)?;
        require_keys_eq!(
            signer_pubkey,
            self.player.key(),
            DiceError::Ed25519Pubkey
        );

        let ix_sig = signature.signature.ok_or(DiceError::Ed25519Signature)?;
        require!(ix_sig.as_slice() == sig, DiceError::Ed25519Signature);

        let msg = signature.message.as_ref().ok_or(DiceError::Ed25519Message)?;
        let expected_msg = self.bet.to_slice();
        require!(
            msg.as_slice() == expected_msg.as_slice(),
            DiceError::Ed25519Message
        );

        Ok(())
    }

    pub fn resolve_bet(&mut self, sig: &[u8], bumps: &ResolveBetBumps) -> Result<()> {
        require!(self.bet.amount > 0, DiceError::InvalidBetAmount);
        require!(
            (1..=100).contains(&self.bet.roll),
            DiceError::InvalidRoll
        );

        require_keys_eq!(self.bet.player, self.player.key(), DiceError::InvalidPlayer);

        self.verify_ed25519_signature(sig)?;

        let h = hash(sig).to_bytes();

        let mut buf = [0u8; 16];
        buf.copy_from_slice(&h[..16]);
        let lower = u128::from_le_bytes(buf);

        buf.copy_from_slice(&h[16..]);
        let upper = u128::from_le_bytes(buf);

        let roll = (lower.wrapping_add(upper) % 100 + 1) as u8;

        if self.bet.roll >= roll {
            require!(
                HOUSE_FEE_BPS < BPS_DENOMINATOR,
                DiceError::InvalidHouseFee
            );

            let payout = self
                .bet
                .amount
                .checked_mul(BPS_DENOMINATOR - HOUSE_FEE_BPS)
                .ok_or(DiceError::Overflow)?
                .checked_div(BPS_DENOMINATOR)
                .ok_or(DiceError::Overflow)?;

            let vault_lamports = self.vault.to_account_info().lamports();
            require!(vault_lamports >= payout, DiceError::VaultInsufficientFunds);

            let house_key = self.house.key();

            let signer_seeds: &[&[&[u8]]] = &[&[
                b"vault",
                house_key.as_ref(),
                &[bumps.vault],
            ]];

            let cpi_context = CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.player.to_account_info(),
                },
                signer_seeds,
            );

            transfer(cpi_context, payout)?;
        }

        Ok(())
    }
}
