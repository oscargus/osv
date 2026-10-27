use serde::{Deserialize, Serialize};

use crate::artifacts::TypeId;

use super::ValueError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegerValue(pub i64);

impl IntegerValue {
    pub fn checked_add(self, right: Self) -> Result<Self, ValueError> {
        self.0
            .checked_add(right.0)
            .map(Self)
            .ok_or(ValueError::Overflow)
    }

    pub fn checked_sub(self, right: Self) -> Result<Self, ValueError> {
        self.0
            .checked_sub(right.0)
            .map(Self)
            .ok_or(ValueError::Overflow)
    }

    pub fn checked_mul(self, right: Self) -> Result<Self, ValueError> {
        self.0
            .checked_mul(right.0)
            .map(Self)
            .ok_or(ValueError::Overflow)
    }

    pub fn checked_neg(self) -> Result<Self, ValueError> {
        self.0.checked_neg().map(Self).ok_or(ValueError::Overflow)
    }

    pub fn checked_div(self, right: Self) -> Result<Self, ValueError> {
        if right.0 == 0 {
            return Err(ValueError::ZeroDivisor);
        }
        self.0
            .checked_div(right.0)
            .map(Self)
            .ok_or(ValueError::Overflow)
    }

    pub fn check_range(self, minimum: i64, maximum: i64) -> Result<Self, ValueError> {
        if self.0 < minimum || self.0 > maximum {
            Err(ValueError::IntegerRange {
                value: self.0,
                minimum,
                maximum,
            })
        } else {
            Ok(self)
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BitValue {
    Zero,
    One,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnumerationValue {
    pub type_id: TypeId,
    pub position: u32,
}

impl EnumerationValue {
    pub fn new(type_id: TypeId, position: u32, cardinality: u32) -> Result<Self, ValueError> {
        let value = Self { type_id, position };
        value.validate(type_id, cardinality)?;
        Ok(value)
    }

    pub fn validate(self, expected: TypeId, cardinality: u32) -> Result<(), ValueError> {
        if self.type_id != expected {
            return Err(ValueError::TypeMismatch {
                expected,
                actual: self.type_id,
            });
        }
        if self.position >= cardinality {
            return Err(ValueError::EnumerationPosition {
                position: self.position,
                cardinality,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RealRecord")]
pub struct RealValue {
    real_bits: u64,
}

#[derive(Deserialize)]
#[serde(rename = "RealValue", deny_unknown_fields)]
struct RealRecord {
    real_bits: u64,
}

impl TryFrom<RealRecord> for RealValue {
    type Error = ValueError;
    fn try_from(record: RealRecord) -> Result<Self, Self::Error> {
        Self::from_bits(record.real_bits)
    }
}

impl RealValue {
    pub fn new(value: f64) -> Result<Self, ValueError> {
        Self::from_bits(value.to_bits())
    }

    pub fn from_bits(real_bits: u64) -> Result<Self, ValueError> {
        if !f64::from_bits(real_bits).is_finite() {
            return Err(ValueError::NonFiniteReal);
        }
        Ok(Self { real_bits })
    }

    pub fn to_bits(self) -> u64 {
        self.real_bits
    }
    pub fn get(self) -> f64 {
        f64::from_bits(self.real_bits)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalValue {
    pub type_id: TypeId,
    pub primary_units: i64,
}

impl PhysicalValue {
    pub fn from_unit(
        type_id: TypeId,
        magnitude: i64,
        primary_units_per_unit: u64,
    ) -> Result<Self, ValueError> {
        if primary_units_per_unit == 0 || primary_units_per_unit > i64::MAX.unsigned_abs() {
            return Err(ValueError::InvalidUnitScale);
        }
        let units = i128::from(magnitude) * i128::from(primary_units_per_unit);
        Ok(Self {
            type_id,
            primary_units: i64::try_from(units).map_err(|_| ValueError::Overflow)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PrimitiveValue {
    Boolean(bool),
    Bit(BitValue),
    Enumeration(EnumerationValue),
    Integer(IntegerValue),
    Real(RealValue),
    Physical(PhysicalValue),
    Time(super::TimeValue),
}
