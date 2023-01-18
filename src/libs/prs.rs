use super::bltn_func::BltnFunc;
use crate::expr::Bltn;
use crate::object::Object;

pub struct PrimeSieve {
    primality: Vec<bool>,
    index: usize,
}

impl PrimeSieve {
    pub fn new(max: usize) -> Self {
        let max = std::cmp::max(2, max + 1);
        let mut primality = Vec::with_capacity(max);
        primality.resize(max, true);
        primality[0] = false;
        primality[1] = false;
        PrimeSieve {
            primality,
            index: 0,
        }
    }
}

impl Iterator for PrimeSieve {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        let p = loop {
            if self.index >= self.primality.len() {
                return None;
            } else if self.primality[self.index] {
                break self.index;
            }
            self.index += 1;
        };

        for i in (2 * p..self.primality.len()).step_by(p) {
            self.primality[i] = false;
        }
        self.index += 1;
        Some(p)
    }
}

pub struct PrimeFactors {
    number: u64,
    prime: u64,
}

impl PrimeFactors {
    pub fn new(number: u64) -> Self {
        PrimeFactors { number, prime: 2 }
    }
}

impl Iterator for PrimeFactors {
    type Item = (u64, u8);
    fn next(&mut self) -> Option<(u64, u8)> {
        if self.number <= 1 {
            return None;
        }
        let next_prime = |p: u64| {
            if p == 2 {
                3
            } else if p == 3 {
                5
            } else if p % 6 == 1 {
                p + 4
            } else if p % 6 == 5 {
                p + 2
            } else {
                unreachable!()
            }
        };

        let factor = loop {
            if self.prime * self.prime > self.number {
                break if self.number > 1 {
                    let n = self.number;
                    self.number = 1;
                    Some((n, 1))
                } else {
                    None
                };
            }

            let mut exp = 0;
            while self.number % self.prime == 0 {
                self.number /= self.prime;
                exp += 1;
            }
            if exp > 0 {
                break Some((self.prime, exp));
            }
            self.prime = next_prime(self.prime);
        };
        self.prime = next_prime(self.prime);
        factor
    }
}

create_bltns! {prs:
    /// prs.is_prime (x: natural) -> bool
    /// Returns whether 'x' is prime
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
        true
    }

    /// prs.primes (max: natural) -> array of naturals
    /// Return array of all primes up to and including 'max'
    /// Example:    'prs.primes 13 == [2, 3, 5, 7, 11, 13]'
    pub fn primes(max: usize) -> Vec<usize>
        { PrimeSieve::new(max).collect() }

    /// prs.prime_factors (x: natural) -> array of [natural, small natural]
    /// Return array of pairs (p, k) where 'p' is a prime factor of 'x'
    /// and 'k' is its associated power.
    /// The pairs are listed in order of the size of 'p'
    /// Example:    'prs.prime_factors (5 * 2^3) == [[2, 3], [5, 1]]'
    pub fn prime_factors(x: u64) -> Vec<(u64, u8)>
        { PrimeFactors::new(x).collect() }



    /// prs.is_sqfree (x: natural) -> bool
    /// Returns whether 'x' is square-free
    pub fn is_sqfree(x: u64) -> bool {
        if x == 0 { return false }
        PrimeFactors::new(x).all(|(_, k)| k == 1)
    }

    /// prs.radical (x: natural) -> natural
    /// Return the smallest square-free number 's'
    /// such that 'x' divides some power of 's'
    pub fn radical(x: u64) -> u64 {
        if x == 0 { return 0 }
        PrimeFactors::new(x).map(|(p, _)| p).product()
    }



    /// prs.euler_totient (x: natural) -> natural
    /// Euler totient function of 'x'
    pub fn euler_totient(x: u64) -> u64 {
        PrimeFactors::new(x).map(|(p, k)|
            (p - 1) * p.pow(k as u32 - 1)
        ).product()
    }

    /// prs.divisors (x: natural) -> array of naturals
    /// Returns the list of the natural numbers that divide 'x'
    pub fn divisors(x: u64) -> Vec<u64> {
        let mut divs = vec![1];
        for (p, k) in PrimeFactors::new(x) {
            let mut new_divs = divs.clone();
            for _ in 0..k {
                for d in divs.iter_mut() { *d *= p }
                new_divs.extend(divs.iter().cloned());
            }
            divs = new_divs;
        }
        divs
    }

    /// prs.divisor_sum (z: small natural) (x: natural) -> natural
    /// Returns the sum of the divisors of 'x' to the 'z'th power
    /// Example:    'prs.divisor_sum z 6 == 1^z + 2^z + 3^z + 6^z'
    pub fn divisor_sum(z: u8, x: u64) -> u64 {
        PrimeFactors::new(x).map(|(p, k)| {
            let ppow = p.pow(z as u32);
            (ppow.pow(k as u32 + 1) - 1) / (ppow - 1)
        }).product()
    }

    /// prs.is_perfect (x: natural) -> bool
    /// Returns whether 'x' is a perfect number
    pub fn is_perfect(x: u64) -> bool { divisor_sum(1, x) == 2 * x }
}
