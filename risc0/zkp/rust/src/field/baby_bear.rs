// Copyright 2022 Risc0, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// ! Baby bear field.
/// ! Support for the base finite field modulo 15*2^27 + 1
use crate::field::{self, Elem as FieldElem};

use core::ops;

use bytemuck::{Pod, Zeroable};

/// The BabyBear class is an element of the finite field F_p, where P is the
/// prime number 15*2^27 + 1. Put another way, Fp is basically integer
/// arithmetic modulo P.
///
/// The `Fp` datatype is the core type of all of the operations done within the
/// zero knowledge proofs, and is the smallest 'addressable' datatype, and the
/// base type of which all composite types are built. In many ways, one can
/// imagine it as the word size of a very strange architecture.
///
/// This specific prime P was chosen to:
/// - Be less than 2^31 so that it fits within a 32 bit word and doesn't
///   overflow on addition.
/// - Otherwise have as large a power of 2 in the factors of P-1 as possible.
///
/// This last property is useful for number theoretical transforms (the fast
/// fourier transform equivelant on finite fields). See NTT.h for details.
///
/// The Fp class wraps all the standard arithmetic operations to make the finite
/// field elements look basically like ordinary numbers (which they mostly are).
#[derive(Eq, PartialEq, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(transparent)]
pub struct Elem(u32);

impl Default for Elem {
    fn default() -> Self {
        Self::ZERO
    }
}

/// The modulus of the field.
const P: u32 = 15 * (1 << 27) + 1;
/// The modulus of the field as a u64.
const P_U64: u64 = P as u64;

impl field::Elem for Elem {
    const ZERO: Self = Elem::new(0);

    const ONE: Self = Elem::new(1);

    /// Compute the multiplicative inverse of `x`, or `1 / x` in finite field
    /// terms. Since `x ^ (P - 1) == 1 % P` for any `x != 0` (as a
    /// consequence of Fermat's little theorem), it follows that `x *
    /// x ^ (P - 2) == 1 % P` for `x != 0`.  That is, `x ^ (P - 2)` is the
    /// multiplicative inverse of `x`. Computed this way, the *inverse* of
    /// zero comes out as zero, which is convenient in many cases, so we
    /// leave it.
    fn inv(self) -> Self {
        self.pow((P - 2) as usize)
    }

    fn random(rng: &mut impl rand::Rng) -> Self {
        // Reject the last modulo-P region of possible uint32_t values, since it's
        // uneven and will only return random values less than (2^32 % P).
        const REJECT_CUTOFF: u32 = (u32::MAX / P) * P;
        let mut val: u32 = rng.gen();

        while val >= REJECT_CUTOFF {
            val = rng.gen();
        }
        Elem::from(val)
    }
}

macro_rules! rou_array {
    [$($x:literal),* $(,)?] => {
        [$(Elem::new($x)),* ]
    }
}

impl field::RootsOfUnity for Elem {
    const MAX_ROU_PO2: usize = 27;

    const ROU_FWD: &'static [Elem] = &rou_array![
        1, 2013265920, 284861408, 1801542727, 567209306, 740045640, 918899846, 1881002012,
        1453957774, 65325759, 1538055801, 515192888, 483885487, 157393079, 1695124103, 2005211659,
        1540072241, 88064245, 1542985445, 1269900459, 1461624142, 825701067, 682402162, 1311873874,
        1164520853, 352275361, 18769, 137
    ];

    const ROU_REV: &'static [Elem] = &rou_array![
        1, 2013265920, 1728404513, 1592366214, 196396260, 1253260071, 72041623, 1091445674,
        145223211, 1446820157, 1030796471, 2010749425, 1827366325, 1239938613, 246299276,
        596347512, 1893145354, 246074437, 1525739923, 1194341128, 1463599021, 704606912, 95395244,
        15672543, 647517488, 584175179, 137728885, 749463956
    ];
}

impl Elem {
    /// Create a new [BabyBear] from a raw integer.
    pub const fn new(x: u32) -> Self {
        Self(x % P)
    }
}

impl ops::Add for Elem {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Elem(add(self.0, rhs.0))
    }
}

impl ops::AddAssign for Elem {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = add(self.0, rhs.0)
    }
}

impl ops::Sub for Elem {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Elem(sub(self.0, rhs.0))
    }
}

impl ops::SubAssign for Elem {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = sub(self.0, rhs.0)
    }
}

impl ops::Mul for Elem {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Elem(mul(self.0, rhs.0))
    }
}

impl ops::MulAssign for Elem {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = mul(self.0, rhs.0)
    }
}

impl ops::Neg for Elem {
    type Output = Self;
    fn neg(self) -> Self {
        Elem(0) - self
    }
}

impl From<Elem> for u32 {
    fn from(x: Elem) -> Self {
        x.0
    }
}

impl From<&Elem> for u32 {
    fn from(x: &Elem) -> Self {
        x.0
    }
}

impl From<Elem> for u64 {
    fn from(x: Elem) -> Self {
        x.0.into()
    }
}

impl From<u32> for Elem {
    fn from(x: u32) -> Self {
        Elem(x % P)
    }
}

impl From<u64> for Elem {
    fn from(x: u64) -> Self {
        Elem((x % P_U64) as u32)
    }
}

fn add(lhs: u32, rhs: u32) -> u32 {
    let x = lhs + rhs;
    return if x >= P { x - P } else { x };
}

fn sub(lhs: u32, rhs: u32) -> u32 {
    let x = lhs.wrapping_sub(rhs);
    return if x > P { x.wrapping_add(P) } else { x };
}

fn mul(lhs: u32, rhs: u32) -> u32 {
    (((lhs as u64) * (rhs as u64)) % P_U64) as u32
}

/// The size of the extension field in elements, 4 in this case.
const EXT_SIZE: usize = 4;

/// Instances of `ExtElem` are elements of a finite field `F_p^4`. They are
/// represented as elements of `F_p[X] / (X^4 - 11)`. Basically, this is a *big*
/// finite field (about `2^128` elements), which is used when the security of
/// various operations depends on the size of the field. It has the field
/// `Elem` as a subfield, which means operations by the two are compatable,
/// which is important. The irreducible polynomial was choosen to be the most
/// simple possible one, `x^4 - B`, where `11` is the smallest `B` which makes
/// the polynomial irreducable.
#[derive(Eq, PartialEq, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(transparent)]
pub struct ExtElem([Elem; EXT_SIZE]);

impl Default for ExtElem {
    fn default() -> Self {
        Self::ZERO
    }
}

impl field::Elem for ExtElem {
    const ZERO: ExtElem = ExtElem::zero();
    const ONE: ExtElem = ExtElem::one();

    /// Generate a random field element uniformly.
    fn random(rng: &mut impl rand::Rng) -> Self {
        Self([
            Elem::random(rng),
            Elem::random(rng),
            Elem::random(rng),
            Elem::random(rng),
        ])
    }

    /// Raise a [ExtElem] to a power of `n`.
    fn pow(self, n: usize) -> Self {
        let mut n = n;
        let mut tot = ExtElem::from(1);
        let mut x = self;
        while n != 0 {
            if n % 2 == 1 {
                tot *= x;
            }
            n = n / 2;
            x *= x;
        }
        tot
    }

    /// Compute the multiplicative inverse of an `ExtElem`.
    fn inv(self) -> Self {
        let a = &self.0;
        // Compute the multiplicative inverse by looking at `ExtElem` as a composite
        // field and using the same basic methods used to invert complex
        // numbers. We imagine that initially we have a numerator of `1`, and a
        // denominator of `a`. `out = 1 / a`; We set `a'` to be a with the first
        // and third components negated. We then multiply the numerator and the
        // denominator by `a'`, producing `out = a' / (a * a')`. By construction
        // `(a * a')` has `0`s in its first and third elements. We call this
        // number, `b` and compute it as follows.
        let mut b0 = a[0] * a[0] + BETA * (a[1] * (a[3] + a[3]) - a[2] * a[2]);
        let mut b2 = a[0] * (a[2] + a[2]) - a[1] * a[1] + BETA * (a[3] * a[3]);
        // Now, we make `b'` by inverting `b2`. When we muliply both sizes by `b'`, we
        // get `out = (a' * b') / (b * b')`.  But by construction `b * b'` is in
        // fact an element of `Elem`, call it `c`.
        let c = b0 * b0 + BETA * b2 * b2;
        // But we can now invert `C` direcly, and multiply by `a' * b'`:
        // `out = a' * b' * inv(c)`
        let ic = c.inv();
        // Note: if c == 0 (really should only happen if in == 0), our
        // 'safe' version of inverse results in ic == 0, and thus out
        // = 0, so we have the same 'safe' behavior for ExtElem.  Oh,
        // and since we want to multiply everything by ic, it's
        // slightly faster to pre-multiply the two parts of b by ic (2
        // multiplies instead of 4).
        b0 *= ic;
        b2 *= ic;
        ExtElem([
            a[0] * b0 + BETA * a[2] * b2,
            -a[1] * b0 + NBETA * a[3] * b2,
            -a[0] * b2 + a[2] * b0,
            a[1] * b2 - a[3] * b0,
        ])
    }
}

impl field::ExtElem for ExtElem {
    const EXT_SIZE: usize = EXT_SIZE;

    type SubElem = Elem;

    fn from_subfield(elem: &Elem) -> Self {
        Self::from([elem.clone(), Elem::ZERO, Elem::ZERO, Elem::ZERO])
    }
}

impl From<[Elem; EXT_SIZE]> for ExtElem {
    fn from(val: [Elem; EXT_SIZE]) -> Self {
        ExtElem(val)
    }
}

const BETA: Elem = Elem::new(11);
const NBETA: Elem = Elem::new(P - 11);

impl ExtElem {
    /// Explicitly construct an ExtElem from parts.
    pub fn new(x0: Elem, x1: Elem, x2: Elem, x3: Elem) -> Self {
        Self([x0, x1, x2, x3])
    }

    /// Create a [ExtElem] from a [Elem].
    pub fn from_fp(x: Elem) -> Self {
        Self([x, Elem::new(0), Elem::new(0), Elem::new(0)])
    }

    /// Create a [ExtElem] from a raw integer.
    pub const fn from_u32(x0: u32) -> Self {
        Self([Elem::new(x0), Elem::new(0), Elem::new(0), Elem::new(0)])
    }

    /// Returns the value zero.
    const fn zero() -> Self {
        Self::from_u32(0)
    }

    /// Returns the value one.
    const fn one() -> Self {
        Self::from_u32(1)
    }

    /// Returns the constant portion of a [Elem].
    pub fn const_part(self) -> Elem {
        self.0[0]
    }

    /// Returns the elements of a [Elem].
    pub fn elems(&self) -> &[Elem] {
        &self.0
    }
}

impl ops::Add for ExtElem {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let mut lhs = self;
        lhs += rhs;
        lhs
    }
}

impl ops::AddAssign for ExtElem {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
        }
    }
}

impl ops::Sub for ExtElem {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let mut lhs = self;
        lhs -= rhs;
        lhs
    }
}

impl ops::SubAssign for ExtElem {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..self.0.len() {
            self.0[i] -= rhs.0[i];
        }
    }
}

/// Implement the simple multiplication case by the subfield Elem.
impl ops::MulAssign<Elem> for ExtElem {
    fn mul_assign(&mut self, rhs: Elem) {
        for i in 0..self.0.len() {
            self.0[i] *= rhs;
        }
    }
}

impl ops::Mul<Elem> for ExtElem {
    type Output = Self;
    fn mul(self, rhs: Elem) -> Self {
        let mut lhs = self;
        lhs *= rhs;
        lhs
    }
}

impl ops::Mul<ExtElem> for Elem {
    type Output = ExtElem;
    fn mul(self, rhs: ExtElem) -> ExtElem {
        rhs * self
    }
}

// Now we get to the interesting case of multiplication. Basically,
// multiply out the polynomial representations, and then reduce module
// `x^4 - B`, which means powers >= 4 get shifted back 4 and
// multiplied by `-beta`. We could write this as a double loops with
// some `if`s and hope it gets unrolled properly, but it's small
// enough to just hand write.
impl ops::MulAssign for ExtElem {
    fn mul_assign(&mut self, rhs: Self) {
        // Rename the element arrays to something small for readability.
        let a = &self.0;
        let b = &rhs.0;
        self.0 = [
            a[0] * b[0] + NBETA * (a[1] * b[3] + a[2] * b[2] + a[3] * b[1]),
            a[0] * b[1] + a[1] * b[0] + NBETA * (a[2] * b[3] + a[3] * b[2]),
            a[0] * b[2] + a[1] * b[1] + a[2] * b[0] + NBETA * (a[3] * b[3]),
            a[0] * b[3] + a[1] * b[2] + a[2] * b[1] + a[3] * b[0],
        ];
    }
}

impl ops::Mul for ExtElem {
    type Output = ExtElem;
    fn mul(self, rhs: ExtElem) -> ExtElem {
        let mut lhs = self;
        lhs *= rhs;
        lhs
    }
}

impl ops::Neg for ExtElem {
    type Output = Self;
    fn neg(self) -> Self {
        ExtElem::ZERO - self
    }
}

impl From<u32> for ExtElem {
    fn from(x: u32) -> Self {
        Self([Elem::from(x), Elem::ZERO, Elem::ZERO, Elem::ZERO])
    }
}

impl From<Elem> for ExtElem {
    fn from(x: Elem) -> Self {
        Self([x, Elem::ZERO, Elem::ZERO, Elem::ZERO])
    }
}

#[cfg(test)]
mod tests {
    use super::field;
    use super::{Elem, ExtElem, P, P_U64};
    use crate::field::Elem as FieldElem;
    use rand::SeedableRng;

    #[test]
    pub fn roots_of_unity() {
        field::test::test_roots_of_unity::<Elem>();
    }

    #[test]
    pub fn field_ops() {
        field::test::test_field_ops::<Elem>(P_U64);
    }

    #[test]
    fn isa_field() {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(2);
        // Pick random sets of 3 elements of ExtElem, and verify they meet the
        // requirements of a field.
        for _ in 0..1_000 {
            let a = ExtElem::random(&mut rng);
            let b = ExtElem::random(&mut rng);
            let c = ExtElem::random(&mut rng);
            // Addition + multiplication commute
            assert_eq!(a + b, b + a);
            assert_eq!(a * b, b * a);
            // Addition + multiplication are associative
            assert_eq!(a + (b + c), (a + b) + c);
            assert_eq!(a * (b * c), (a * b) * c);
            // Distributive property
            assert_eq!(a * (b + c), a * b + a * c);
            // Inverses
            if a != ExtElem::ZERO {
                assert_eq!(a.inv() * a, ExtElem::from(1));
            }
            assert_eq!(ExtElem::ZERO - a, -a);
            assert_eq!(a + (-a), ExtElem::ZERO);
        }
    }

    #[test]
    fn inv() {
        // Smoke test for inv
        assert_eq!(Elem(5).inv() * Elem(5), Elem(1));
    }

    #[test]
    fn pow() {
        // Smoke tests for pow
        assert_eq!(Elem(5).pow(0), Elem(1));
        assert_eq!(Elem(5).pow(1), Elem(5));
        assert_eq!(Elem(5).pow(2), Elem(25));
        // Mathematica says PowerMod[5, 1000, 15*2^27 + 1] == 589699054
        assert_eq!(Elem(5).pow(1000), Elem(589699054));
        assert_eq!(Elem(5).pow((P - 2) as usize) * Elem(5), Elem(1));
        assert_eq!(Elem(5).pow((P - 1) as usize), Elem(1));
    }

    #[test]
    fn compare_native() {
        // Compare core operations against simple % P implementations
        let mut rng = rand::rngs::SmallRng::seed_from_u64(2);
        for _ in 0..100_000 {
            let fa = Elem::random(&mut rng);
            let fb = Elem::random(&mut rng);
            let a: u64 = fa.into();
            let b: u64 = fb.into();
            assert_eq!(fa + fb, Elem::from(a + b));
            assert_eq!(fa - fb, Elem::from(a + (P_U64 - b)));
            assert_eq!(fa * fb, Elem::from(a * b));
        }
    }
}
