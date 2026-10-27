use serde::{Deserialize, Serialize};

use super::{IndexRange, MAX_VECTOR_BITS, PackedBinary, ValueError};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LogicState {
    U,
    X,
    Zero,
    One,
    Z,
    W,
    L,
    H,
    DontCare,
}

impl LogicState {
    pub fn binary(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            _ => None,
        }
    }

    pub fn from_binary(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }

    pub fn symbol(self) -> char {
        match self {
            Self::U => 'U',
            Self::X => 'X',
            Self::Zero => '0',
            Self::One => '1',
            Self::Z => 'Z',
            Self::W => 'W',
            Self::L => 'L',
            Self::H => 'H',
            Self::DontCare => '-',
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::U => 0,
            Self::X => 1,
            Self::Zero => 2,
            Self::One => 3,
            Self::Z => 4,
            Self::W => 5,
            Self::L => 6,
            Self::H => 7,
            Self::DontCare => 8,
        }
    }

    fn from_code(code: u8) -> Result<Self, ValueError> {
        match code {
            0 => Ok(Self::U),
            1 => Ok(Self::X),
            2 => Ok(Self::Zero),
            3 => Ok(Self::One),
            4 => Ok(Self::Z),
            5 => Ok(Self::W),
            6 => Ok(Self::L),
            7 => Ok(Self::H),
            8 => Ok(Self::DontCare),
            _ => Err(ValueError::VectorEncoding("unknown logic-state code")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogicStorage {
    Binary(PackedBinary),
    States { nibbles: Vec<u8>, nonbinary: u64 },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "LogicRecord")]
pub struct LogicVector {
    width: u64,
    storage: LogicStorage,
}

#[derive(Deserialize, Serialize)]
enum LogicRecord {
    Binary(PackedBinary),
    States(Vec<LogicState>),
}

impl TryFrom<LogicRecord> for LogicVector {
    type Error = ValueError;
    fn try_from(record: LogicRecord) -> Result<Self, Self::Error> {
        match record {
            LogicRecord::Binary(value) => Ok(Self::from_packed(value)),
            LogicRecord::States(states) => Self::from_states(&states),
        }
    }
}

impl Serialize for LogicVector {
    fn serialize<Serializer: serde::Serializer>(
        &self,
        serializer: Serializer,
    ) -> Result<Serializer::Ok, Serializer::Error> {
        let record = match &self.storage {
            LogicStorage::Binary(value) => LogicRecord::Binary(value.clone()),
            LogicStorage::States { nonbinary: 0, .. } => {
                LogicRecord::Binary(self.packed().map_err(serde::ser::Error::custom)?)
            }
            LogicStorage::States { .. } => {
                LogicRecord::States(self.states().map_err(serde::ser::Error::custom)?)
            }
        };
        record.serialize(serializer)
    }
}

impl PartialEq for LogicVector {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && match (&self.storage, &other.storage) {
                (LogicStorage::Binary(left), LogicStorage::Binary(right)) => left == right,
                _ => (0..self.width).all(|offset| self.state(offset) == other.state(offset)),
            }
    }
}

impl Eq for LogicVector {}

fn nibble_location(offset: u64) -> Result<(usize, u32), ValueError> {
    Ok((
        usize::try_from(offset / 2).map_err(|_| ValueError::Overflow)?,
        if offset.is_multiple_of(2) { 0 } else { 4 },
    ))
}

fn write_nibble(bytes: &mut [u8], offset: u64, state: LogicState) -> Result<(), ValueError> {
    let (index, shift) = nibble_location(offset)?;
    let byte = bytes.get_mut(index).ok_or(ValueError::VectorEncoding(
        "logic storage is shorter than width",
    ))?;
    *byte = (*byte & !(0x0f << shift)) | (state.code() << shift);
    Ok(())
}

fn buffer(width: u64) -> Result<Vec<u8>, ValueError> {
    if width > MAX_VECTOR_BITS {
        return Err(ValueError::VectorWidth {
            width,
            maximum: MAX_VECTOR_BITS,
        });
    }
    let bytes = usize::try_from(width.div_ceil(2)).map_err(|_| ValueError::Overflow)?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(bytes)
        .map_err(|_| ValueError::AllocationLimit)?;
    buffer.resize(bytes, 0);
    Ok(buffer)
}

impl LogicVector {
    pub fn from_packed(value: PackedBinary) -> Self {
        Self {
            width: value.width(),
            storage: LogicStorage::Binary(value),
        }
    }

    pub fn from_states(states: &[LogicState]) -> Result<Self, ValueError> {
        let width = u64::try_from(states.len()).map_err(|_| ValueError::Overflow)?;
        if width > MAX_VECTOR_BITS {
            return Err(ValueError::VectorWidth {
                width,
                maximum: MAX_VECTOR_BITS,
            });
        }
        if states.iter().all(|state| state.binary().is_some()) {
            let mut value = PackedBinary::from_u64(width, 0)?;
            for (offset, state) in states.iter().enumerate() {
                value.set_bit(
                    width - 1 - u64::try_from(offset).map_err(|_| ValueError::Overflow)?,
                    state
                        .binary()
                        .ok_or(ValueError::VectorEncoding("nonbinary state"))?,
                )?;
            }
            return Ok(Self::from_packed(value));
        }
        let mut nibbles = buffer(width)?;
        let mut nonbinary = 0;
        for (offset, state) in states.iter().enumerate() {
            write_nibble(
                &mut nibbles,
                u64::try_from(offset).map_err(|_| ValueError::Overflow)?,
                *state,
            )?;
            if state.binary().is_none() {
                nonbinary += 1;
            }
        }
        Ok(Self {
            width,
            storage: LogicStorage::States { nibbles, nonbinary },
        })
    }

    pub fn width(&self) -> u64 {
        self.width
    }
    pub fn storage(&self) -> &LogicStorage {
        &self.storage
    }

    pub fn state(&self, offset: u64) -> Result<LogicState, ValueError> {
        if offset >= self.width {
            return Err(ValueError::StorageOffset {
                offset,
                length: self.width,
            });
        }
        match &self.storage {
            LogicStorage::Binary(value) => {
                Ok(LogicState::from_binary(value.bit(self.width - 1 - offset)?))
            }
            LogicStorage::States { nibbles, .. } => {
                let (index, shift) = nibble_location(offset)?;
                LogicState::from_code(
                    (nibbles.get(index).ok_or(ValueError::VectorEncoding(
                        "logic storage is shorter than width",
                    ))? >> shift)
                        & 0x0f,
                )
            }
        }
    }

    pub fn states(&self) -> Result<Vec<LogicState>, ValueError> {
        let mut states = Vec::new();
        states
            .try_reserve_exact(usize::try_from(self.width).map_err(|_| ValueError::Overflow)?)
            .map_err(|_| ValueError::AllocationLimit)?;
        for offset in 0..self.width {
            states.push(self.state(offset)?);
        }
        Ok(states)
    }

    pub fn promote(&mut self) -> Result<(), ValueError> {
        if matches!(self.storage, LogicStorage::States { .. }) {
            return Ok(());
        }
        let mut nibbles = buffer(self.width)?;
        for offset in 0..self.width {
            write_nibble(&mut nibbles, offset, self.state(offset)?)?;
        }
        self.storage = LogicStorage::States {
            nibbles,
            nonbinary: 0,
        };
        Ok(())
    }

    fn packed(&self) -> Result<PackedBinary, ValueError> {
        let mut value = PackedBinary::from_u64(self.width, 0)?;
        for offset in 0..self.width {
            let bit = self
                .state(offset)?
                .binary()
                .ok_or(ValueError::VectorEncoding(
                    "nonbinary state cannot be packed",
                ))?;
            value.set_bit(self.width - 1 - offset, bit)?;
        }
        Ok(value)
    }

    pub fn try_repack(&mut self) -> Result<bool, ValueError> {
        match &self.storage {
            LogicStorage::Binary(_) => Ok(true),
            LogicStorage::States { nonbinary: 0, .. } => {
                self.storage = LogicStorage::Binary(self.packed()?);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn set_state(&mut self, offset: u64, state: LogicState) -> Result<(), ValueError> {
        let previous = self.state(offset)?;
        if let (LogicStorage::Binary(value), Some(bit)) = (&mut self.storage, state.binary()) {
            return value.set_bit(self.width - 1 - offset, bit);
        }
        self.promote()?;
        if let LogicStorage::States { nibbles, nonbinary } = &mut self.storage {
            write_nibble(nibbles, offset, state)?;
            if previous.binary().is_none() {
                *nonbinary -= 1;
            }
            if state.binary().is_none() {
                *nonbinary += 1;
            }
        }
        self.try_repack()?;
        Ok(())
    }

    pub fn logical_state(&self, range: IndexRange, index: i64) -> Result<LogicState, ValueError> {
        let actual = range.length()?;
        if actual != self.width {
            return Err(ValueError::WidthMismatch {
                expected: self.width,
                actual,
            });
        }
        self.state(range.offset(index)?)
    }

    pub fn slice(&self, start: u64, length: u64) -> Result<Self, ValueError> {
        let end = start.checked_add(length).ok_or(ValueError::Overflow)?;
        if start > self.width || end > self.width {
            return Err(ValueError::StorageOffset {
                offset: end,
                length: self.width,
            });
        }
        let mut states = Vec::new();
        states
            .try_reserve_exact(usize::try_from(length).map_err(|_| ValueError::Overflow)?)
            .map_err(|_| ValueError::AllocationLimit)?;
        for offset in start..end {
            states.push(self.state(offset)?);
        }
        Self::from_states(&states)
    }
}
