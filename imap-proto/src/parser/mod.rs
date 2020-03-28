use crate::types::Response;
use nom::IResult;

pub use self::rfc3501::*;
pub use self::rfc5464::*;

pub mod core;

mod rfc3501;
mod rfc4551;
mod rfc5464;

pub type ParseResult<'a> = IResult<&'a [u8], Response<'a>>;
