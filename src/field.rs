use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

pub const MODULUS: u64 = 998_244_353;
const PRIMITIVE_ROOT: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fp(pub u64);

impl Fp {
    pub fn new(value: u64) -> Self {
        Self(value % MODULUS)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn one() -> Self {
        Self(1)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn pow(self, mut exp: u64) -> Self {
        let mut base = self;
        let mut acc = Fp::one();
        while exp > 0 {
            if exp & 1 == 1 {
                acc *= base;
            }
            base *= base;
            exp >>= 1;
        }
        acc
    }

    pub fn inv(self) -> Self {
        self.pow(MODULUS - 2)
    }

    pub fn root_of_unity(log_n: usize) -> Self {
        let n = 1u64 << log_n;
        assert!((MODULUS - 1) % n == 0);
        Fp::new(PRIMITIVE_ROOT).pow((MODULUS - 1) / n)
    }
}

impl Display for Fp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Fp {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        let mut sum = self.0 + rhs.0;
        if sum >= MODULUS {
            sum -= MODULUS;
        }
        Fp(sum)
    }
}

impl AddAssign for Fp {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Fp {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Fp(self.0 - rhs.0)
        } else {
            Fp(self.0 + MODULUS - rhs.0)
        }
    }
}

impl SubAssign for Fp {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Fp {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let z = (self.0 as u128 * rhs.0 as u128) % (MODULUS as u128);
        Fp(z as u64)
    }
}

impl MulAssign for Fp {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

pub fn to_field_vec(raw: &[u64]) -> Vec<Fp> {
    raw.iter().copied().map(Fp::new).collect()
}
