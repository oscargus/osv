use serde::{Deserialize, Serialize};

use crate::artifacts::TypeId;

use super::ValueError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexRange {
    pub left: i64,
    pub right: i64,
    pub direction: Direction,
}

impl IndexRange {
    pub fn is_null(self) -> bool {
        match self.direction {
            Direction::Ascending => self.left > self.right,
            Direction::Descending => self.left < self.right,
        }
    }

    pub fn length(self) -> Result<u64, ValueError> {
        if self.is_null() {
            return Ok(0);
        }
        let length = (i128::from(self.left) - i128::from(self.right)).abs() + 1;
        u64::try_from(length).map_err(|_| ValueError::Overflow)
    }

    pub fn offset(self, index: i64) -> Result<u64, ValueError> {
        self.length()?;
        let contains = match self.direction {
            Direction::Ascending => self.left <= index && index <= self.right,
            Direction::Descending => self.right <= index && index <= self.left,
        };
        if !contains {
            return Err(ValueError::IndexBounds { index, range: self });
        }
        Ok(index.abs_diff(self.left))
    }

    pub fn index_at(self, offset: u64) -> Result<i64, ValueError> {
        let length = self.length()?;
        if offset >= length {
            return Err(ValueError::StorageOffset { offset, length });
        }
        let index = match self.direction {
            Direction::Ascending => i128::from(self.left) + i128::from(offset),
            Direction::Descending => i128::from(self.left) - i128::from(offset),
        };
        i64::try_from(index).map_err(|_| ValueError::Overflow)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RangeConstraint {
    Unconstrained,
    Constrained(IndexRange),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArrayDimension {
    pub index_type: TypeId,
    pub range: RangeConstraint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArrayShape {
    pub dimensions: Vec<ArrayDimension>,
}

fn element_count(ranges: &[IndexRange]) -> Result<u64, ValueError> {
    if ranges.iter().any(|range| range.is_null()) {
        return Ok(0);
    }
    ranges.iter().try_fold(1u64, |count, range| {
        count
            .checked_mul(range.length()?)
            .ok_or(ValueError::Overflow)
    })
}

impl ArrayShape {
    pub fn ranges(&self) -> Result<Vec<IndexRange>, ValueError> {
        if self.dimensions.is_empty() {
            return Err(ValueError::ArrayDimensions {
                expected: 1,
                actual: 0,
            });
        }
        self.dimensions
            .iter()
            .map(|dimension| match dimension.range {
                RangeConstraint::Constrained(range) => Ok(range),
                RangeConstraint::Unconstrained => Err(ValueError::UnconstrainedArray),
            })
            .collect()
    }

    pub fn element_count(&self) -> Result<u64, ValueError> {
        element_count(&self.ranges()?)
    }

    pub fn storage_offset(&self, indices: &[i64]) -> Result<u64, ValueError> {
        if indices.len() != self.dimensions.len() {
            return Err(ValueError::ArrayDimensions {
                expected: self.dimensions.len(),
                actual: indices.len(),
            });
        }
        let ranges = self.ranges()?;
        element_count(&ranges)?;
        if let Some((dimension, range)) =
            ranges.iter().enumerate().find(|(_, range)| range.is_null())
        {
            return Err(ValueError::IndexBounds {
                index: indices[dimension],
                range: *range,
            });
        }
        ranges
            .iter()
            .zip(indices)
            .try_fold(0u64, |offset, (range, index)| {
                let local = range.offset(*index)?;
                offset
                    .checked_mul(range.length()?)
                    .and_then(|offset| offset.checked_add(local))
                    .ok_or(ValueError::Overflow)
            })
    }

    pub fn indices_at(&self, offset: u64) -> Result<Vec<i64>, ValueError> {
        let ranges = self.ranges()?;
        let length = element_count(&ranges)?;
        if offset >= length {
            return Err(ValueError::StorageOffset { offset, length });
        }
        let mut remaining = offset;
        let mut indices = Vec::with_capacity(ranges.len());
        for range in ranges.iter().rev() {
            let length = range.length()?;
            indices.push(range.index_at(remaining % length)?);
            remaining /= length;
        }
        indices.reverse();
        Ok(indices)
    }
}
