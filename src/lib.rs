//! Address Lookup Table Program

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// TODO: Program-test will not overwrite existing built-ins
// See <PR>
// solana_program::declare_id!("AddressLookupTab1e1111111111111111111111111");
solana_program::declare_id!("AaoNx79M6YE3DcXfrRN4nmBcQvQPqdpowi6uEESuJdnm");

/// The definition of address lookup table accounts.
///
/// As used by the `crate::message::v0` message format.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddressLookupTableAccount {
    pub key: solana_program::pubkey::Pubkey,
    pub addresses: Vec<solana_program::pubkey::Pubkey>,
}
