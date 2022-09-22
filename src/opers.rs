use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary { Neg }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary { Add, Sub, Mul, Div, Mod, Pow }

type Prec = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc { Left, Right }

impl Unary {
    pub fn prec(&self) -> Prec {
        match self {
            Unary::Neg => 50,
        }
    }
    
    pub fn symbol(&self) -> &str {
        match self {
            Unary::Neg => "-",
        }
    }
    
    pub const OPERS: [Self; 1] = [
        Unary::Neg,
    ];
}

impl Binary {
    pub fn prec(&self) -> Prec {
        match self {
            Binary::Add => 50,
            Binary::Sub => 50,
            Binary::Mul => 75,
            Binary::Div => 75,
            Binary::Mod => 75,
            Binary::Pow => 100,
        }
    }
    
    pub fn assoc(&self) -> Assoc {
        match self {
            Binary::Add => Assoc::Left,
            Binary::Sub => Assoc::Left,
            Binary::Mul => Assoc::Left,
            Binary::Div => Assoc::Left,
            Binary::Mod => Assoc::Left,
            Binary::Pow => Assoc::Right,
        }
    }
    
    pub fn symbol(&self) -> &str {
        match self {
            Binary::Add => "+",
            Binary::Sub => "-",
            Binary::Mul => "*",
            Binary::Div => "/",
            Binary::Mod => "%",
            Binary::Pow => "^",
        }
    }
    
    pub const OPERS: [Self; 6] = [
        Binary::Add, Binary::Sub,
        Binary::Mul, Binary::Div,
        Binary::Mod,
        Binary::Pow,
    ];
}

impl FromStr for Unary {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Unary::OPERS.iter().copied()
        .filter(|op| s.starts_with(op.symbol()))
        .max_by_key(|op| op.symbol().len()).ok_or(())
    }
}

impl FromStr for Binary {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Binary::OPERS.iter().copied()
        .filter(|op| s.starts_with(op.symbol()))
        .max_by_key(|op| op.symbol().len()).ok_or(())
    }
}

