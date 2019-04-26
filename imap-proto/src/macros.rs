macro_rules! paren_delimited (
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        delimited!($i, char!('('), $submac!($($args)*), char!(')'))
    });
    ($i:expr, $f:expr) => (
        paren_delimited!($i, call!($f));
    );
);

macro_rules! parenthesized_list (
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        paren_delimited!($i, separated_list!(char!(' '), $submac!($($args)*)))
    });
    ($i:expr, $f:expr) => (
        parenthesized_nonempty_list!($i, call!($f));
    );
);

macro_rules! parenthesized_nonempty_list (
    ($i:expr, $submac:ident!( $($args:tt)* )) => ({
        paren_delimited!($i, separated_nonempty_list!(char!(' '), $submac!($($args)*)))
    });
    ($i:expr, $f:expr) => (
        parenthesized_nonempty_list!($i, call!($f));
    );
);
