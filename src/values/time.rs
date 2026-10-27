use serde::{Deserialize, Serialize};

use super::ValueError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimeUnit {
    Femtosecond,
    Picosecond,
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
}

impl TimeUnit {
    pub const fn femtoseconds(self) -> i64 {
        match self {
            Self::Femtosecond => 1,
            Self::Picosecond => 1_000,
            Self::Nanosecond => 1_000_000,
            Self::Microsecond => 1_000_000_000,
            Self::Millisecond => 1_000_000_000_000,
            Self::Second => 1_000_000_000_000_000,
            Self::Minute => 60_000_000_000_000_000,
            Self::Hour => 3_600_000_000_000_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimeResolution {
    #[default]
    Femtosecond,
    Picosecond,
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
}

impl TimeResolution {
    pub const fn femtoseconds(self) -> u64 {
        match self {
            Self::Femtosecond => 1,
            Self::Picosecond => 1_000,
            Self::Nanosecond => 1_000_000,
            Self::Microsecond => 1_000_000_000,
            Self::Millisecond => 1_000_000_000_000,
            Self::Second => 1_000_000_000_000_000,
        }
    }

    pub const fn max_ticks(self) -> u64 {
        i64::MAX.unsigned_abs() / self.femtoseconds()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimeValue(pub i64);

impl TimeValue {
    pub fn from_unit(magnitude: i64, unit: TimeUnit) -> Result<Self, ValueError> {
        magnitude
            .checked_mul(unit.femtoseconds())
            .map(Self)
            .ok_or(ValueError::Overflow)
    }

    pub fn to_ticks(self, resolution: TimeResolution) -> Result<SimulationTime, ValueError> {
        let femtoseconds = u64::try_from(self.0).map_err(|_| ValueError::NegativeDelay)?;
        let scale = resolution.femtoseconds();
        if !femtoseconds.is_multiple_of(scale) {
            return Err(ValueError::ResolutionLoss {
                femtoseconds: self.0,
                resolution_fs: scale,
            });
        }
        SimulationTime::new(femtoseconds / scale, resolution)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "TimeRecord")]
pub struct SimulationTime {
    ticks: u64,
    resolution: TimeResolution,
}

#[derive(Deserialize)]
#[serde(rename = "SimulationTime", deny_unknown_fields)]
struct TimeRecord {
    ticks: u64,
    resolution: TimeResolution,
}

impl TryFrom<TimeRecord> for SimulationTime {
    type Error = ValueError;
    fn try_from(record: TimeRecord) -> Result<Self, Self::Error> {
        Self::new(record.ticks, record.resolution)
    }
}

impl SimulationTime {
    pub fn new(ticks: u64, resolution: TimeResolution) -> Result<Self, ValueError> {
        if ticks > resolution.max_ticks() {
            return Err(ValueError::Overflow);
        }
        Ok(Self { ticks, resolution })
    }

    pub fn ticks(self) -> u64 {
        self.ticks
    }
    pub fn resolution(self) -> TimeResolution {
        self.resolution
    }

    pub fn to_time(self) -> Result<TimeValue, ValueError> {
        let value = self
            .ticks
            .checked_mul(self.resolution.femtoseconds())
            .ok_or(ValueError::Overflow)?;
        i64::try_from(value)
            .map(TimeValue)
            .map_err(|_| ValueError::Overflow)
    }

    pub fn checked_advance(self, delay: TimeValue) -> Result<Self, ValueError> {
        let delay = delay.to_ticks(self.resolution)?;
        let ticks = self
            .ticks
            .checked_add(delay.ticks)
            .ok_or(ValueError::Overflow)?;
        Self::new(ticks, self.resolution)
    }
}
