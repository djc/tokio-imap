use crate::types::Response;
use nom::{branch::alt, IResult};

pub mod core;

pub mod bodystructure;
pub mod rfc3501;
pub mod rfc4315;
pub mod rfc4551;
pub mod rfc5161;
pub mod rfc5464;
pub mod rfc7162;

#[cfg(test)]
mod tests;

pub fn parse_response(msg: &[u8]) -> ParseResult {
    alt((
        rfc3501::continue_req,
        rfc3501::response_data,
        rfc3501::response_tagged,
    ))(msg)
}

pub type ParseResult<'a> = IResult<&'a [u8], Response<'a>>;
