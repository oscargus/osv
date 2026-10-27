use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Serialize};

use super::{IndexRange, ValueError};

pub const MAX_VECTOR_BITS: u64 = 32_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackedStorage {
    Small(u64),
    Big(BigUint),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "BinaryRecord")]
pub struct PackedBinary {
    width: u64,
    storage: PackedStorage,
}

#[derive(Deserialize, Serialize)]
#[serde(rename = "PackedBinary", deny_unknown_fields)]
struct BinaryRecord {
    width: u64,
    storage: StorageRecord,
}

#[derive(Deserialize, Serialize)]
enum StorageRecord {
    Small(u64),
    Big { limbs_le: Vec<u32> },
}

impl Serialize for PackedBinary {
    fn serialize<Serializer: serde::Serializer>(
        &self,
        serializer: Serializer,
    ) -> Result<Serializer::Ok, Serializer::Error> {
        let storage = match &self.storage {
            PackedStorage::Small(value) => StorageRecord::Small(*value),
            PackedStorage::Big(value) => StorageRecord::Big {
                limbs_le: value.to_u32_digits(),
            },
        };
        BinaryRecord {
            width: self.width,
            storage,
        }
        .serialize(serializer)
    }
}

impl TryFrom<BinaryRecord> for PackedBinary {
    type Error = ValueError;

    fn try_from(record: BinaryRecord) -> Result<Self, Self::Error> {
        check_width(record.width)?;
        match record.storage {
            StorageRecord::Small(value) if record.width <= 64 => {
                Self::from_u64(record.width, value)
            }
            StorageRecord::Big { limbs_le } if record.width > 64 => {
                if limbs_le.last() == Some(&0) {
                    return Err(ValueError::VectorEncoding("redundant high zero limb"));
                }
                if u64::try_from(limbs_le.len()).map_err(|_| ValueError::Overflow)?
                    > record.width.div_ceil(32)
                {
                    return Err(ValueError::VectorEncoding(
                        "magnitude limb count exceeds width",
                    ));
                }
                Self::from_biguint(record.width, BigUint::new(limbs_le))
            }
            _ => Err(ValueError::VectorEncoding(
                "storage kind does not match width",
            )),
        }
    }
}

fn check_width(width: u64) -> Result<(), ValueError> {
    if width > MAX_VECTOR_BITS {
        Err(ValueError::VectorWidth {
            width,
            maximum: MAX_VECTOR_BITS,
        })
    } else {
        Ok(())
    }
}

impl PackedBinary {
    pub fn from_u64(width: u64, value: u64) -> Result<Self, ValueError> {
        check_width(width)?;
        let bits = 64 - u64::from(value.leading_zeros());
        if bits > width {
            return Err(ValueError::VectorValueTooWide { width, bits });
        }
        let storage = if width <= 64 {
            PackedStorage::Small(value)
        } else {
            PackedStorage::Big(BigUint::from(value))
        };
        Ok(Self { width, storage })
    }

    pub fn from_biguint(width: u64, value: BigUint) -> Result<Self, ValueError> {
        check_width(width)?;
        if value.bits() > width {
            return Err(ValueError::VectorValueTooWide {
                width,
                bits: value.bits(),
            });
        }
        if width <= 64 {
            let value = value.iter_u64_digits().next().unwrap_or(0);
            Self::from_u64(width, value)
        } else {
            Ok(Self {
                width,
                storage: PackedStorage::Big(value),
            })
        }
    }

    pub fn masked_from_u64(width: u64, value: u64) -> Result<Self, ValueError> {
        check_width(width)?;
        if width > 64 {
            return Self::from_u64(width, value);
        }
        let mask = match width {
            0 => 0,
            64 => u64::MAX,
            1..=63 => (1u64 << width) - 1,
            _ => return Err(ValueError::Overflow),
        };
        Ok(Self {
            width,
            storage: PackedStorage::Small(value & mask),
        })
    }

    pub fn masked_from_biguint(width: u64, value: BigUint) -> Result<Self, ValueError> {
        check_width(width)?;
        if value.bits() <= width {
            return Self::from_biguint(width, value);
        }
        let shift = usize::try_from(width).map_err(|_| ValueError::Overflow)?;
        let mask = (BigUint::from(1u8) << shift) - BigUint::from(1u8);
        Self::from_biguint(width, value & mask)
    }

    pub fn width(&self) -> u64 {
        self.width
    }
    pub fn storage(&self) -> &PackedStorage {
        &self.storage
    }

    pub fn unsigned_value(&self) -> BigUint {
        match &self.storage {
            PackedStorage::Small(value) => BigUint::from(*value),
            PackedStorage::Big(value) => value.clone(),
        }
    }

    pub fn signed_value(&self) -> Result<BigInt, ValueError> {
        let value = BigInt::from(self.unsigned_value());
        if self.width == 0 || !self.bit(self.width - 1)? {
            return Ok(value);
        }
        let shift = usize::try_from(self.width).map_err(|_| ValueError::Overflow)?;
        Ok(value - (BigInt::from(1u8) << shift))
    }

    pub fn bit(&self, from_right: u64) -> Result<bool, ValueError> {
        if from_right >= self.width {
            return Err(ValueError::StorageOffset {
                offset: from_right,
                length: self.width,
            });
        }
        Ok(match &self.storage {
            PackedStorage::Small(value) => value & (1u64 << from_right) != 0,
            PackedStorage::Big(value) => value.bit(from_right),
        })
    }

    pub fn set_bit(&mut self, from_right: u64, bit: bool) -> Result<(), ValueError> {
        if from_right >= self.width {
            return Err(ValueError::StorageOffset {
                offset: from_right,
                length: self.width,
            });
        }
        match &mut self.storage {
            PackedStorage::Small(value) => {
                let mask = 1u64 << from_right;
                if bit {
                    *value |= mask;
                } else {
                    *value &= !mask;
                }
            }
            PackedStorage::Big(value) => value.set_bit(from_right, bit),
        }
        Ok(())
    }

    pub fn logical_bit(&self, range: IndexRange, index: i64) -> Result<bool, ValueError> {
        let actual = range.length()?;
        if actual != self.width {
            return Err(ValueError::WidthMismatch {
                expected: self.width,
                actual,
            });
        }
        let offset = range.offset(index)?;
        self.bit(self.width - 1 - offset)
    }
}
