use nom::IResult;
use crate::types::Response;

pub mod core;

pub mod rfc3501;
pub mod rfc4551;
pub mod rfc5464;

pub type ParseResult<'a> = IResult<&'a [u8], Response<'a>>;
