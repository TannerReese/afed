
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

macro_rules! recover {
    ($doc:expr, $parse:expr, $recover:expr) => { match $parse {
        Ok(good) => Ok(good),
        Err(None) => $recover,
        Err(Some(err)) => { $doc.add_error(err); $recover },
    }};
}

macro_rules! revert {
    ($pos:ident : $parse:expr) => {{
        let start = *$pos;
        let res = $parse;
        if res.is_err() { *$pos = start; }
        res
    }};
}

macro_rules! peek {
    ($pos:ident : $parse:expr) => {{
        let start = *$pos;
        let res = $parse;
        *$pos = start;
        res
    }}
}

macro_rules! opt {
    ($parse:expr) => { match $parse {
        Ok(good) => Ok(Some(good)),
        Err(None) => Ok(None),
        Err(Some(errs)) => Err(Some(errs)),
    }};
}

macro_rules! ign {
    ($parse:expr) => { $parse.map(|_| ()) };
}

macro_rules! seq {
    ($pos:ident : $before:expr, $parse:expr) => {
        seq!($pos: $before, $parse, Ok(()))
    };
    ($pos:ident : , $parse:expr, $after:expr) => {
        seq!($pos: Ok(()), $parse, $after)
    };
    ($pos:ident : $before:expr, $parse:expr, $after:expr) => {
        revert!($pos: loop {
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

macro_rules! tuple {
    ($pos:ident : $($parse:expr),+) => {
        revert!($pos: loop {
            break Ok(($(match $parse {
                Ok(good) => good,
                Err(err) => break Err(err),
            }),+))
        })
    };
}

macro_rules! many0 {
    ($parse:expr) => { many0!($parse, Ok(())) };
    ($parse:expr, $sep:expr) => {{
        let mut results = Vec::new();
        loop {
            match $parse {
                Ok(val) => results.push(val),
                Err(None) => break Ok(results),
                Err(Some(err)) => break Err(Some(err)),
            }

            match $sep {
                Ok(_) => {},
                Err(None) => break Ok(results),
                Err(Some(err)) => break Err(Some(err)),
            }
        }
    }};
}

macro_rules! many1 {
    ($($arg:tt)*) => {
        many0!($($arg)*).and_then(|results|
            if results.len() == 0 { Err(None) }
            else { Ok(results) }
        )
    }
}

