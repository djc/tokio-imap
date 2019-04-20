macro_rules! opt_opt(
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        use nom::lib::std::result::Result::*;
        use nom::lib::std::option::Option::*;
        use nom::Err;

        let i_ = $i.clone();
        match $submac!(i_, $($args)*) {
            Ok((i,o))          => Ok((i, o)),
            Err(Err::Error(_)) => Ok(($i, None)),
            Err(e)             => Err(e),
        }
    });
    ($i:expr, $f:expr) => (
        opt_opt!($i, call!($f));
    );
);
