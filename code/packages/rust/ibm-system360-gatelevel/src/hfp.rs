//! Gate-network hexadecimal floating-point arithmetic.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{not_gate, or_gate, xor_gate};

fn bits(value: u128) -> [u8; 128] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn value(bits: &[u8]) -> u128 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

fn add(a: u128, b: u128, carry: u8) -> (u128, u8) {
    let result = ripple_carry_adder_with_carry(&bits(a), &bits(b), carry);
    (value(&result.sum), result.carry_out)
}

fn invert(input: u128) -> u128 {
    value(&bits(input).map(not_gate))
}

fn subtract(a: u128, b: u128) -> (u128, u8) {
    add(a, invert(b), 1)
}

fn zero(value: u128) -> u8 {
    not_gate(bits(value).into_iter().fold(0, or_gate))
}

fn compare(a: u128, b: u128) -> u8 {
    let (difference, carry) = subtract(a, b);
    if zero(difference) == 1 {
        0
    } else if carry == 0 {
        1
    } else {
        2
    }
}

fn multiply(a: u128, b: u128) -> Option<u128> {
    let multiplier = bits(b);
    let mut result = 0_u128;
    for (bit, selected) in multiplier.into_iter().enumerate() {
        if selected == 1 {
            let shifted = a.checked_shl(bit as u32)?;
            let (next, carry) = add(result, shifted, 0);
            if carry != 0 {
                return None;
            }
            result = next;
        }
    }
    Some(result)
}

fn divide(numerator: u128, denominator: u128) -> Option<u128> {
    if denominator == 0 {
        return None;
    }
    let numerator_bits = bits(numerator);
    let mut remainder = 0_u128;
    let mut quotient = 0_u128;
    for bit in (0..128).rev() {
        remainder = (remainder << 1) | u128::from(numerator_bits[bit]);
        let (difference, carry) = subtract(remainder, denominator);
        if carry == 1 {
            remainder = difference;
            quotient |= 1_u128 << bit;
        }
    }
    Some(quotient)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GateHfp {
    negative: bool,
    exponent: i16,
    fraction: u128,
}

impl GateHfp {
    pub(crate) fn decode(raw: u64, long: bool) -> Self {
        Self {
            negative: raw >> 63 != 0,
            exponent: ((raw >> 56) & 0x7f) as i16 - 64,
            fraction: if long {
                u128::from(raw & 0x00ff_ffff_ffff_ffff)
            } else {
                u128::from((raw >> 32) & 0x00ff_ffff)
            },
        }
    }

    pub(crate) fn encode(self, long: bool) -> Option<u64> {
        if self.fraction == 0 {
            return Some(0);
        }
        if !(-64..=63).contains(&self.exponent) {
            return None;
        }
        let prefix = (u64::from(self.negative) << 63) | (((self.exponent + 64) as u64) << 56);
        Some(if long {
            prefix | self.fraction as u64
        } else {
            prefix | ((self.fraction as u64) << 32)
        })
    }

    pub(crate) fn with_complemented_sign(self) -> Self {
        Self {
            negative: !self.negative,
            ..self
        }
    }

    pub(crate) fn condition_code(self) -> u8 {
        if self.fraction == 0 {
            0
        } else if self.negative {
            1
        } else {
            2
        }
    }

    fn normalize(
        negative: bool,
        mut fraction: u128,
        mut exponent: i16,
        digits: u32,
    ) -> Option<Self> {
        if fraction == 0 {
            return Some(Self {
                negative: false,
                exponent: 0,
                fraction: 0,
            });
        }
        let limit = 1_u128 << (digits * 4);
        let floor = 1_u128 << ((digits - 1) * 4);
        while fraction >= limit {
            fraction >>= 4;
            exponent += 1;
        }
        while fraction < floor {
            fraction <<= 4;
            exponent -= 1;
        }
        (-64..=63).contains(&exponent).then_some(Self {
            negative,
            exponent,
            fraction,
        })
    }

    pub(crate) fn add(self, other: Self, digits: u32) -> Option<Self> {
        let exponent = self.exponent.max(other.exponent);
        let align = |value: Self| {
            let places = (exponent - value.exponent) as u32;
            if places >= 32 {
                0
            } else {
                value.fraction >> (places * 4)
            }
        };
        let a = align(self);
        let b = align(other);
        if self.negative == other.negative {
            Self::normalize(self.negative, add(a, b, 0).0, exponent, digits)
        } else {
            match compare(a, b) {
                0 => Self::normalize(false, 0, exponent, digits),
                1 => Self::normalize(other.negative, subtract(b, a).0, exponent, digits),
                _ => Self::normalize(self.negative, subtract(a, b).0, exponent, digits),
            }
        }
    }

    pub(crate) fn halve(self, digits: u32) -> Option<Self> {
        Self::normalize(self.negative, self.fraction >> 1, self.exponent, digits)
    }

    pub(crate) fn multiply(self, other: Self, digits: u32) -> Option<Self> {
        if self.fraction == 0 || other.fraction == 0 {
            return Self::normalize(false, 0, 0, digits);
        }
        let fraction = multiply(self.fraction, other.fraction)? >> (digits * 4);
        Self::normalize(
            xor_gate(u8::from(self.negative), u8::from(other.negative)) == 1,
            fraction,
            self.exponent + other.exponent,
            digits,
        )
    }

    pub(crate) fn divide(self, other: Self, digits: u32) -> Option<Self> {
        if other.fraction == 0 {
            return None;
        }
        if self.fraction == 0 {
            return Self::normalize(false, 0, 0, digits);
        }
        let numerator = self.fraction.checked_shl(digits * 4)?;
        let fraction = divide(numerator, other.fraction)?;
        Self::normalize(
            xor_gate(u8::from(self.negative), u8::from(other.negative)) == 1,
            fraction,
            self.exponent - other.exponent,
            digits,
        )
    }

    pub(crate) fn compare(self, other: Self) -> u8 {
        if self.fraction == 0 && other.fraction == 0 {
            return 0;
        }
        if self.fraction == 0 {
            return if other.negative { 2 } else { 1 };
        }
        if other.fraction == 0 {
            return if self.negative { 1 } else { 2 };
        }
        if self.negative != other.negative {
            return if self.negative { 1 } else { 2 };
        }
        let normalized = |mut value: Self| {
            while value.fraction < (1_u128 << 124) {
                value.fraction <<= 4;
                value.exponent -= 1;
            }
            value
        };
        let a = normalized(self);
        let b = normalized(other);
        let magnitude = if a.exponent == b.exponent {
            compare(a.fraction, b.fraction)
        } else if a.exponent < b.exponent {
            1
        } else {
            2
        };
        if self.negative {
            match magnitude {
                1 => 2,
                2 => 1,
                other => other,
            }
        } else {
            magnitude
        }
    }
}
