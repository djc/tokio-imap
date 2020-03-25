use crate::types::Response;
use nom::IResult;

pub mod core;
pub mod rfc3501;
pub mod rfc4551;
pub mod rfc5464;

// TODO: change to ParseResult<'a, T> for Command?
pub type ParseResult<'a> = IResult<&'a [u8], Response<'a>>;
