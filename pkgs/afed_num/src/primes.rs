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

pub fn euler_totient(x: u64) -> u64 {
    PrimeFactors::new(x)
        .map(|(p, k)| (p - 1) * p.pow(k as u32 - 1))
        .product()
}
