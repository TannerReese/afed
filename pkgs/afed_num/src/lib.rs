// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use afed_objects::{declare_pkg, number::Number, Object};
use modulo::Modulo;
use primes::{PrimeFactors, PrimeSieve};

pub mod modulo;
pub mod primes;

declare_pkg! {num:
    /// num.is_prime (x: natural) -> bool
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

    /// num.primes (max: natural) -> array of naturals
    /// Return array of all primes up to and including 'max'
    /// Example:    'num.primes 13 == [2, 3, 5, 7, 11, 13]'
    pub fn primes(max: usize) -> Vec<usize>
        { PrimeSieve::new(max).collect() }

    /// num.prime_factors (x: natural) -> array of [natural, small natural]
    /// Return array of pairs (p, k) where 'p' is a prime factor of 'x'
    /// and 'k' is its associated power.
    /// The pairs are listed in order of the size of 'p'
    /// Example:    'num.prime_factors (5 * 2^3) == [[2, 3], [5, 1]]'
    pub fn prime_factors(x: u64) -> Vec<(u64, u8)>
        { PrimeFactors::new(x).collect() }



    /// num.is_sqfree (x: natural) -> bool
    /// Returns whether 'x' is square-free
    pub fn is_sqfree(x: u64) -> bool {
        if x == 0 { return false }
        PrimeFactors::new(x).all(|(_, k)| k == 1)
    }

    /// num.radical (x: natural) -> natural
    /// Return the smallest square-free number 's'
    /// such that 'x' divides some power of 's'
    pub fn radical(x: u64) -> u64 {
        if x == 0 { return 0 }
        PrimeFactors::new(x).map(|(p, _)| p).product()
    }



    /// num.euler_totient (x: natural) -> natural
    /// Euler totient function of 'x'
    pub fn euler_totient(x: u64) -> u64 {
        primes::euler_totient(x)
    }

    /// num.divisors (x: natural) -> array of naturals
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

    /// num.divisor_sum (z: small natural) (x: natural) -> natural
    /// Returns the sum of the divisors of 'x' to the 'z'th power
    /// Example:    'num.divisor_sum z 6 == 1^z + 2^z + 3^z + 6^z'
    pub fn divisor_sum(z: u8, x: u64) -> u64 {
        PrimeFactors::new(x).map(|(p, k)| {
            let ppow = p.pow(z as u32);
            (ppow.pow(k as u32 + 1) - 1) / (ppow - 1)
        }).product()
    }

    /// num.is_perfect (x: natural) -> bool
    /// Returns whether 'x' is a perfect number
    pub fn is_perfect(x: u64) -> bool { divisor_sum(1, x) == 2 * x }



    /// num.Mod (m: integer) -> modulo
    /// Return residue class '1 (mod m)'
    /// Can be used to generate all residue classes
    /// so '6 * num.Mod 15' represents '6 (mod 15)'
    #[allow(non_snake_case)]
    #[global]
    fn Mod(m: Number) -> Result<Modulo, &'static str> { match m {
        Number::Ratio(0, 1) => Err("Modulo can't be zero"),
        Number::Ratio(m, 1) => Ok(Modulo::from(1, m.unsigned_abs())),
        _ => Err("Modulo must be a non-zero integer"),
    }}
}
