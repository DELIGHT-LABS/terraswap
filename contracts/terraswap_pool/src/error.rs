use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Alread exist. ({0})")]
    AlreadyExsit(String),

    #[error("Unknown reply id. ({0})")]
    UnknownReplyId(u64),
}
