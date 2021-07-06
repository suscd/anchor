use anchor_lang::prelude::*;
use anchor_spl::dex;
use serum_dex::instruction::MarketInstruction;
use serum_dex::state::OpenOrders;
use solana_program::instruction::Instruction;
use solana_program::program;
use solana_program::system_program;
use std::mem::size_of;

/// This demonstrates how to create "permissioned markets" on Serum. A
/// permissioned market is a regular Serum market with an additional
/// open orders authority, which must sign every transaction to create or
/// close an open orders account.
///
/// In practice, what this means is that one can create a program that acts
/// as this authority *and* that marks its own PDAs as the *owner* of all
/// created open orders accounts, making the program the sole arbiter over
/// who can trade on a given market.
///
/// For example, this example forces all trades that execute on this market
/// to set the referral to a hardcoded address, i.e., `fee_owner::ID`.
#[program]
pub mod permissioned_markets {
    use super::*;

    /// Fallback function to relay calls to the serum DEX.
    ///
    /// For instructions requiring an open orders authority, checks for
    /// a user signature and then swaps the account info for one controlled
    /// by the program.
    ///
    /// Note: the "authority" of each open orders account is the account
    ///       itself, since it's a PDA.
    #[access_control(is_serum(program_id))]
    pub fn dex_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        // Decode the dex instruction.
        let ix = MarketInstruction::unpack(data).ok_or_else(|| ErrorCode::InvalidInstruction)?;

        // Swap the user's account, which is in the open orders authority
        // position, for the program's PDA (the real authority).
        let mut acc_infos = accounts.to_vec();
        let (market, user) = match ix {
            MarketInstruction::InitOpenOrders => {
                assert!(accounts.len() >= 4);

                let (market, user) = {
                    let market = &acc_infos[2];
                    let user = &acc_infos[1];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[1] = acc_infos[0].clone();

                (market, user)
            }
            MarketInstruction::CloseOpenOrders => {
                assert!(accounts.len() >= 4);

                let (market, user) = {
                    let market = &acc_infos[3];
                    let user = &acc_infos[1];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[1] = acc_infos[0].clone();

                (market, user)
            }
            MarketInstruction::NewOrderV3(_) => {
                assert!(accounts.len() >= 13);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[7];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[7] = acc_infos[1].clone();

                (market, user)
            }
            MarketInstruction::CancelOrderV2(_) => {
                assert!(accounts.len() >= 6);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[4];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[4] = acc_infos[3].clone();

                (market, user)
            }
            MarketInstruction::CancelOrderByClientIdV2(_) => {
                assert!(accounts.len() >= 6);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[4];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[4] = acc_infos[3].clone();

                (market, user)
            }
            MarketInstruction::SettleFunds => {
                assert!(accounts.len() >= 10);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[2];
                    let referral = &accounts[10];

                    if referral.key != &fee_owner::ID {
                        return Err(ErrorCode::InvalidReferral.into());
                    }
                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[2] = acc_infos[1].clone();

                (market, user)
            }
            _ => return Err(ErrorCode::InvalidInstruction.into()),
        };

        // CPI to the dex.
        let accounts = acc_infos
            .iter()
            .map(|acc| AccountMeta {
                pubkey: *acc.key,
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            })
            .collect();
        let ix = Instruction {
            data: data.to_vec(),
            accounts,
            program_id: dex::ID,
        };
        let seeds = open_orders_authority! {
            program = program_id,
            market = market,
            authority = user
        };
        let init_seeds = open_orders_init_authority! {
            program = program_id,
            market = market
        };
        program::invoke_signed(&ix, &acc_infos, &[seeds, init_seeds])
    }

    /// Creates an open orders account owned by the program on behalf of the
    /// user. The user is defined by the authority.
    ///
    /// Note: this is just a convenience API so that we can use an auto
    ///       generated client, since `@project-serum/serum` doesn't currently
    ///       have this api in the npm package. Once it does this can be removed.
    pub fn init_account(ctx: Context<InitAccount>, bump: u8, bump_init: u8) -> Result<()> {
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        let seeds = open_orders_authority! {
            program = ctx.program_id,
            market = ctx.accounts.market.key,
            authority = ctx.accounts.authority.key,
            bump = bump
        };
        let init_seeds = open_orders_init_authority! {
            program = ctx.program_id,
            market = ctx.accounts.market.key,
            bump = bump_init
        };
        dex::init_open_orders(cpi_ctx.with_signer(&[seeds, init_seeds]))?;
        Ok(())
    }

    /// Closes an open orders account on behalf of the user to retrieve the
    /// rent exemption SOL back.
    ///
    /// Note: this is just a convenience API so that we can use an auto
    ///       generated client, since `@project-serum/serum` doesn't currently
    ///       have this api in the npm package. Once it does this can be removed.
    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        let seeds = open_orders_authority! {
            program = ctx.program_id,
            market = ctx.accounts.market.key,
            authority = ctx.accounts.authority.key
        };
        dex::close_open_orders(cpi_ctx.with_signer(&[seeds]))?;
        Ok(())
    }
}

// Accounts context.

#[derive(Accounts)]
#[instruction(bump: u8, bump_init: u8)]
pub struct InitAccount<'info> {
    #[account(seeds = [b"open-orders-init", market.key.as_ref(), &[bump_init]])]
    pub open_orders_init_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"open-orders", market.key.as_ref(), authority.key.as_ref(), &[bump]],
        payer = authority,
        owner = dex::ID,
        space = size_of::<OpenOrders>() + SERUM_PADDING,
    )]
    pub open_orders: AccountInfo<'info>,
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    pub dex_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    open_orders: AccountInfo<'info>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    market: AccountInfo<'info>,
    destination: AccountInfo<'info>,
    dex_program: AccountInfo<'info>,
    rent: AccountInfo<'info>,
}

// CpiContext transformations.

impl<'info> From<&InitAccount<'info>>
    for CpiContext<'_, '_, '_, 'info, dex::InitOpenOrders<'info>>
{
    fn from(accs: &InitAccount<'info>) -> Self {
        let accounts = dex::InitOpenOrders {
            open_orders: accs.open_orders.clone(),
            authority: accs.open_orders.clone(),
            market: accs.market.clone(),
            rent: accs.rent.to_account_info(),
        };
        let program = accs.dex_program.clone();
        CpiContext::new(program, accounts)
    }
}

impl<'info> From<&CloseAccount<'info>>
    for CpiContext<'_, '_, '_, 'info, dex::CloseOpenOrders<'info>>
{
    fn from(accs: &CloseAccount<'info>) -> Self {
        let accounts = dex::CloseOpenOrders {
            open_orders: accs.open_orders.clone(),
            authority: accs.authority.clone(),
            destination: accs.destination.clone(),
            market: accs.market.clone(),
        };
        let program = accs.dex_program.clone();
        CpiContext::new(program, accounts)
    }
}

// Access control modifiers.

fn is_serum<'info>(program_id: &Pubkey) -> Result<()> {
    if program_id != &dex::ID {
        return Err(ErrorCode::InvalidDexPid.into());
    }
    Err(ErrorCode::InvalidInstruction.into())
}

// Error.

#[error]
pub enum ErrorCode {
    #[msg("Program ID does not match the Serum DEX")]
    InvalidDexPid,
    #[msg("Invalid instruction given")]
    InvalidInstruction,
    #[msg("Invalid referral address given")]
    InvalidReferral,
    #[msg("The user didn't sign")]
    UnauthorizedUser,
}

// Macros.

/// Returns the seeds used for creating the open orders account PDA.
#[macro_export]
macro_rules! open_orders_authority {
    (program = $program:expr, market = $market:expr, authority = $authority:expr, bump = $bump:expr) => {
        &[
            b"open-orders".as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[$bump],
        ]
    };
    (program = $program:expr, market = $market:expr, authority = $authority:expr) => {
        &[
            b"open-orders".as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[Pubkey::find_program_address(
                &[
                    b"open-orders".as_ref(),
                    $market.as_ref(),
                    $authority.as_ref(),
                ],
                $program,
            )
            .1],
        ]
    };
}

/// Returns the seeds used for the open orders init authority.
/// This is the account that must sign to create a new open orders account on
/// the DEX market.
#[macro_export]
macro_rules! open_orders_init_authority {
    (program = $program:expr, market = $market:expr) => {
        &[
            b"open-orders-init".as_ref(),
            $market.as_ref(),
            &[Pubkey::find_program_address(
                &[b"open-orders-init".as_ref(), $market.as_ref()],
                $program,
            )
            .1],
        ]
    };
    (program = $program:expr, market = $market:expr, bump = $bump:expr) => {
        &[b"open-orders-init".as_ref(), $market.as_ref(), &[$bump]]
    };
}

// Constants.

// Padding added to every serum account.
//
// b"serum".len() + b"padding".len().
const SERUM_PADDING: usize = 12;

/// The address that will receive all fees for all markets controlled by this
/// program. Note: this is a dummy address. Do not use in production.
pub mod fee_owner {
    solana_program::declare_id!("2k1bb16Hu7ocviT2KC3wcCgETtnC8tEUuvFBH4C5xStG");
}
