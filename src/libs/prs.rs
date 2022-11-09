use std::collections::HashMap;
use super::bltn_func::BltnFunc;
use crate::object::Object;

pub fn is_prime(x: u64) -> bool {
    match x {
        0 | 1 => return false,
        2 | 3 => return true,
        _ => {},
    }

    if x % 2 == 0 || x % 3 == 0 { return false }

    for p in (5..).step_by(6) {
        if p * p > x { break }
        if x % p == 0 || x % (p + 2) == 0 { return false }
    }
    return true
}

pub struct PrimeSieve {
    primality: Vec<bool>,
    index: usize,
}

pub fn prime_sieve(max: usize) -> PrimeSieve {
    let max = std::cmp::max(2, max + 1);
    let mut primality = Vec::with_capacity(max);
    primality.resize(max, true);
    primality[0] = false;
    primality[1] = false;
    PrimeSieve { primality, index: 0 }
}

impl Iterator for PrimeSieve {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        let p = loop {
            if self.index >= self.primality.len() { return None }
            else if self.primality[self.index] { break self.index }
            self.index += 1;
        };

        for i in (2 * p .. self.primality.len()).step_by(p) {
            self.primality[i] = false;
        }
        self.index += 1;
        return Some(p)
    }
}

pub struct PrimeFactors {
    number: u64,
    prime: u64,
}

impl Iterator for PrimeFactors {
    type Item = (u64, u8);
    fn next(&mut self) -> Option<(u64, u8)> {
        if self.number <= 1 { return None }
        let next_prime = |p: u64| {
            if p == 2 { 3 }
            else if p == 3 { 5 }
            else if p % 6 == 1 { p + 4 }
            else if p % 6 == 5 { p + 2 }
            else { unreachable!() }
        };

        let factor = loop {
            if self.prime * self.prime > self.number {
                break if self.number > 1 {
                    let n = self.number;
                    self.number = 1;
                    Some((n, 1))
                } else { None };
            }

            let mut exp = 0;
            while self.number % self.prime == 0 {
                self.number /= self.prime;
                exp += 1;
            }
            if exp > 0 { break Some((self.prime, exp)); }
            self.prime = next_prime(self.prime);
        };
        self.prime = next_prime(self.prime);
        factor
    }
}

pub fn prime_factors(x: u64) -> PrimeFactors {
    PrimeFactors { number: x, prime: 2 }
}

pub fn is_sqfree(x: u64) -> bool {
    if x == 0 { return false }
    prime_factors(x).all(|(_, k)| k == 1)
}

pub fn radical(x: u64) -> u64 {
    if x == 0 { return 0 }
    prime_factors(x).map(|(p, _)| p).product()
}

pub fn euler_totient(x: u64) -> u64 {
    prime_factors(x).map(|(p, k)|
        (p - 1) * p.pow(k as u32 - 1)
    ).product()
}

pub fn divisors(x: u64) -> Vec<u64> {
    let mut divs = vec![1];
    for (p, k) in prime_factors(x) {
        let mut new_divs = divs.clone();
        for _ in 0..k {
            for d in divs.iter_mut() { *d *= p }
            new_divs.extend(divs.iter().cloned());
        }
        divs = new_divs;
    }
    divs
}

pub fn sum_of_divisors(z: u8, x: u64) -> u64 {
    prime_factors(x).map(|(p, k)| {
        let ppow = p.pow(z as u32);
        (ppow.pow(k as u32 + 1) - 1) / (ppow - 1)
    }).product()
}

pub fn is_perfect(x: u64) -> bool {
    sum_of_divisors(1, x) == 2 * x
}


pub fn make_bltns() -> Object {
    let mut prs = HashMap::new();
    def_bltn!(prs.is_prime(x: u64) = is_prime(x).into());
    def_bltn!(prs.primes(x: usize) =
        prime_sieve(x).map(|p| p.into()).collect()
    );
    def_bltn!(prs.prime_factors(x: u64) = prime_factors(x).map(|(p, k)|
        vec![p, k as u64].into()
    ).collect());

    def_bltn!(prs.is_sqfree(x: u64) = is_sqfree(x).into());
    def_bltn!(prs.radical(x: u64) = radical(x).into());

    def_bltn!(prs.euler_totient(x: u64) = euler_totient(x).into());
    def_bltn!(prs.divisors(n: u64) =
        divisors(n).into_iter().map(|d| d.into()).collect()
    );
    def_bltn!(prs.divisor_sum(z: u8, n: u64) = sum_of_divisors(z, n).into());
    def_bltn!(prs.is_perfect(x: u64) = is_perfect(x).into());
    prs.into()
}

