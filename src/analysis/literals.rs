use std::{error::Error, fmt};

use num_bigint::{BigInt, BigUint};

use crate::{
    syntax::{SyntaxToken, TokenKind},
    values::IntegerValue,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiteralError {
    Invalid,
    UnsupportedReal,
    ResourceLimit,
    IntegerRange,
}

impl fmt::Display for LiteralError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => formatter.write_str("invalid integer literal"),
            Self::UnsupportedReal => formatter
                .write_str("real/fractional literal is outside the initial expression subset"),
            Self::ResourceLimit => {
                formatter.write_str("numeric literal exceeds the exact-decoding resource limit")
            }
            Self::IntegerRange => {
                formatter.write_str("integer literal is outside the signed 64-bit base range")
            }
        }
    }
}
impl Error for LiteralError {}

pub fn integer_magnitude(token: &SyntaxToken) -> Result<BigUint, LiteralError> {
    if token.kind() != TokenKind::AbstractLiteral {
        return Err(LiteralError::Invalid);
    }
    let text = token.text();
    if text.len() > 131_072 {
        return Err(LiteralError::ResourceLimit);
    }
    let text = text.replace('_', "");
    if text.contains('.') {
        return Err(LiteralError::UnsupportedReal);
    }
    let (radix, digits, exponent) = if let Some((radix, remaining)) = text.split_once('#') {
        let radix = radix.parse::<u32>().map_err(|_| LiteralError::Invalid)?;
        if !(2..=16).contains(&radix) {
            return Err(LiteralError::Invalid);
        }
        let (digits, suffix) = remaining.split_once('#').ok_or(LiteralError::Invalid)?;
        (radix, digits, suffix)
    } else if let Some(position) = text.find(['e', 'E']) {
        (10, &text[..position], &text[position..])
    } else {
        (10, text.as_str(), "")
    };
    let exponent = if exponent.is_empty() {
        0
    } else {
        let exponent = exponent
            .strip_prefix(['e', 'E'])
            .ok_or(LiteralError::Invalid)?;
        if exponent.starts_with('-') {
            return Err(LiteralError::Invalid);
        }
        exponent
            .strip_prefix('+')
            .unwrap_or(exponent)
            .parse::<u32>()
            .map_err(|_| LiteralError::ResourceLimit)?
    };
    if exponent > 1_000_000 {
        return Err(LiteralError::ResourceLimit);
    }
    let magnitude = BigUint::parse_bytes(digits.as_bytes(), radix).ok_or(LiteralError::Invalid)?;
    Ok(magnitude * BigUint::from(radix).pow(exponent))
}

pub fn checked_integer(magnitude: BigUint, negative: bool) -> Result<IntegerValue, LiteralError> {
    let value = BigInt::from(magnitude);
    i64::try_from(if negative { -value } else { value })
        .map(IntegerValue)
        .map_err(|_| LiteralError::IntegerRange)
}
