#[cfg(not(target_os = "solana"))]
use solana_program::message::AddressLoaderError;
use spl_program_error::*;

#[spl_program_error]
pub enum AddressLookupError {
    /// Attempted to lookup addresses from a table that does not exist
    #[error("Attempted to lookup addresses from a table that does not exist")]
    LookupTableAccountNotFound,

    /// Attempted to lookup addresses from an account owned by the wrong program
    #[error("Attempted to lookup addresses from an account owned by the wrong program")]
    InvalidAccountOwner,

    /// Attempted to lookup addresses from an invalid account
    #[error("Attempted to lookup addresses from an invalid account")]
    InvalidAccountData,

    /// Address lookup contains an invalid index
    #[error("Address lookup contains an invalid index")]
    InvalidLookupIndex,

    // Note: The legacy built-in ALT program maps all bincode errors to
    // `InstructionError::GenericError`. Since this error is deprecated, it
    // shouldn't be migrated to `ProgramError`, like the other errors added in
    // this PR: https://github.com/solana-labs/solana/pull/35113.
    // Instead, this would be the only change in the program's ABI: a different
    // error code for failed serialization/deserialization.
    /// Failed to serialize or deserialize lookup table
    #[error("Failed to serialize or deserialize lookup table")]
    SerializationError,
}

#[cfg(not(target_os = "solana"))]
impl From<AddressLookupError> for AddressLoaderError {
    fn from(err: AddressLookupError) -> Self {
        match err {
            AddressLookupError::LookupTableAccountNotFound => Self::LookupTableAccountNotFound,
            AddressLookupError::InvalidAccountOwner => Self::InvalidAccountOwner,
            AddressLookupError::InvalidAccountData => Self::InvalidAccountData,
            AddressLookupError::InvalidLookupIndex => Self::InvalidLookupIndex,
            // TODO: Maybe add this to `AddressLoaderError`?
            AddressLookupError::SerializationError => Self::InvalidAccountData,
        }
    }
}
