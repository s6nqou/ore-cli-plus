use std::fmt::Display;

use ore::error::OreError;
use solana_client::client_error::ClientError;
use solana_sdk::{instruction::InstructionError, transaction::TransactionError};

#[derive(Debug)]
pub enum Error {
    ClientError(ClientError),
    OreError(OreError),
    CliError(CliError),
}

impl From<ClientError> for Error {
    fn from(value: ClientError) -> Self {
        match value.get_transaction_error() {
            Some(TransactionError::InstructionError(_, InstructionError::Custom(e))) => match e {
                0 => Error::OreError(OreError::NotStarted),
                1 => Error::OreError(OreError::NeedsReset),
                2 => Error::OreError(OreError::ResetTooEarly),
                3 => Error::OreError(OreError::HashInvalid),
                4 => Error::OreError(OreError::DifficultyNotSatisfied),
                5 => Error::OreError(OreError::BusRewardsInsufficient),
                6 => Error::OreError(OreError::ClaimTooLarge),
                _ => Error::ClientError(value),
            },
            _ => Error::ClientError(value),
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::ClientError(client_error) => client_error.to_string(),
            Error::OreError(ore_error) => ore_error.to_string(),
            Error::CliError(cli_error) => cli_error.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum CliError {
    TransactionNotLanded,
    LockError,
    WorksEmpty,
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
