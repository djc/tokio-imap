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

macro_rules! paren_delimited(
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        delimited!($i, char!('('), $submac!($($args)*), char!(')'))
    });
    ($i:expr, $f:expr) => (
        paren_delimited!($i, call!($f));
    );
);

macro_rules! paren_list(
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        paren_delimited!($i, separated_nonempty_list!(opt!(space), $submac!($($args)*)))
    });
    ($i:expr, $f:expr) => (
        paren_list!($i, call!($f));
    );
);
