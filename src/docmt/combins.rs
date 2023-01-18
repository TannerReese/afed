/* Each expression, $parse, should attempt to parse a symbol.
 * alt! will perform these in sequence until one of them
 * succeeds or unrecoverably errors.
 */
macro_rules! alt {
    ($($parse:expr),+) => { loop {
        $(match $parse {
            Ok(good) => break Ok(good),
            Err(Some(err)) => break Err(Some(err)),
            Err(None) => {},
        })+
        break Err(None);
    }};
}

/* `$doc` should evaluate to a `Docmt`.
 * `recover!` attempts to execute `$parse`
 * if it fails `$recover` is run.
 */
macro_rules! recover {
    ($doc:expr, $parse:expr, $recover:expr) => {
        match $parse {
            Ok(good) => Ok(good),
            Err(None) => $recover,
            Err(Some(err)) => {
                $doc.add_error(err);
                $recover
            }
        }
    };
}

// Reverts affects of `$parse` when it doesn't succeed
macro_rules! revert {
    ($context:ident : $parse:expr) => {{
        let start = *$context;
        let res = $parse;
        if res.is_err() {
            *$context = start;
        }
        res
    }};
}

// Parses `$parse` without effecting `$context`
macro_rules! peek {
    ($context:ident : $parse:expr) => {{
        let start = *$context;
        let res = $parse;
        *$context = start;
        res
    }};
}

// Optionally parses `$parse` succeeding when nothing is found
macro_rules! opt {
    ($parse:expr) => {
        match $parse {
            Ok(good) => Ok(Some(good)),
            Err(None) => Ok(None),
            Err(Some(errs)) => Err(Some(errs)),
        }
    };
}

// Ignore return value of `$parse` returning () instead
macro_rules! ign {
    ($parse:expr) => {
        $parse.map(|_| ())
    };
}

// Parse `$parse` optionally delimited by `$before` and `$after`
macro_rules! seq {
    ($context:ident : $before:expr, $parse:expr) => {
        seq!($context: $before, $parse, Ok(()))
    };
    ($context:ident : , $parse:expr, $after:expr) => {
        seq!($context: Ok(()), $parse, $after)
    };
    ($context:ident : $before:expr, $parse:expr, $after:expr) => {
        revert!($context: loop {
            match $before {
                Err(err) => break Err(err),
                _ => {},
            }

            let value = $parse;
            if value.is_err() { break value; }

            match $after {
                Err(err) => break Err(err),
                _ => {},
            }
            break value;
        })
    };
}

/* Parses sequence of `$parse` operating on `$context`
 * and creates a tuple from the results if all are successful
 */
macro_rules! tuple {
    ($context:ident : $($parse:expr),+) => {
        revert!($context: loop {
            break Ok(($(match $parse {
                Ok(good) => good,
                Err(err) => break Err(err),
            }),+))
        })
    };
}

/* Parse `$parse` as many times as possible (perhaps zero)
 * and create a vector of the results
 */
macro_rules! many0 {
    ($parse:expr) => {
        many0!($parse, Ok(()))
    };
    ($parse:expr, $sep:expr) => {{
        let mut results = Vec::new();
        loop {
            match $parse {
                Ok(val) => results.push(val),
                Err(None) => break Ok(results),
                Err(Some(err)) => break Err(Some(err)),
            }

            match $sep {
                Ok(_) => {}
                Err(None) => break Ok(results),
                Err(Some(err)) => break Err(Some(err)),
            }
        }
    }};
}

// Same as `many0`, but parses `$parse` at least once
macro_rules! many1 {
    ($($arg:tt)*) => {
        many0!($($arg)*).and_then(|results|
            if results.len() == 0 { Err(None) }
            else { Ok(results) }
        )
    }
}
