#[cfg(not(target_os = "solana"))]
use solana_program::message::AddressLoaderError;
use {solana_program::program_error::ProgramError, spl_program_error::*};

// The legacy built-in ALT program maps all bincode errors to
// `InstructionError::GenericError`.
// Since this error is deprecated, it shouldn't be migrated to `ProgramError`,
// like the other errors added in this PR:
// https://github.com/solana-labs/solana/pull/35113.
// Instead, we should perhaps consider adding a `ProgramError::BincodeIoError`
// or generalizing the `ProgramError::BorshIoError` to `ProgramError::IoError`.
// In either case, we'll need to implement `Into<ProgramError>` for
// `bincode::Error`.
// In the meantime, this is a temporary solution.
pub trait MapToProgramIoError<T> {
    fn map_to_program_io_error(self) -> Result<T, ProgramError>;
}
impl<T> MapToProgramIoError<T> for Result<T, bincode::Error> {
    fn map_to_program_io_error(self) -> Result<T, ProgramError> {
        self.map_err(|e| ProgramError::BorshIoError(e.to_string()))
    }
}

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
}

#[cfg(not(target_os = "solana"))]
impl From<AddressLookupError> for AddressLoaderError {
    fn from(err: AddressLookupError) -> Self {
        match err {
            AddressLookupError::LookupTableAccountNotFound => Self::LookupTableAccountNotFound,
            AddressLookupError::InvalidAccountOwner => Self::InvalidAccountOwner,
            AddressLookupError::InvalidAccountData => Self::InvalidAccountData,
            AddressLookupError::InvalidLookupIndex => Self::InvalidLookupIndex,
        }
    }
}
