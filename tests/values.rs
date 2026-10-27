#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;

use osv::{artifacts::TypeId, values::*};

#[test]
fn primitive_integer_overflow_is_explicit() {
    assert_eq!(
        IntegerValue(i64::MAX).checked_add(IntegerValue(1)),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(i64::MIN).checked_add(IntegerValue(-1)),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(2).checked_add(IntegerValue(3)),
        Ok(IntegerValue(5))
    );
}

#[test]
fn primitive_integer_boundaries_division_and_ranges_are_checked() {
    assert_eq!(
        IntegerValue(i64::MIN).checked_sub(IntegerValue(1)),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(i64::MAX).checked_mul(IntegerValue(2)),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(i64::MIN).checked_neg(),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(i64::MIN).checked_div(IntegerValue(-1)),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        IntegerValue(1).checked_div(IntegerValue(0)),
        Err(ValueError::ZeroDivisor)
    );
    assert_eq!(
        IntegerValue(-7).checked_div(IntegerValue(3)),
        Ok(IntegerValue(-2))
    );
    assert!(IntegerValue(-1).check_range(0, i64::MAX).is_err());
    assert!(IntegerValue(0).check_range(1, i64::MAX).is_err());
    assert_eq!(
        IntegerValue(i64::MIN).check_range(i64::MIN, i64::MAX),
        Ok(IntegerValue(i64::MIN))
    );
}

#[test]
fn primitive_enum_positions_preserve_declared_type_identity() {
    let value = EnumerationValue::new(TypeId(3), 1, 2).unwrap();
    assert!(value.validate(TypeId(4), 2).is_err());
    assert!(EnumerationValue::new(TypeId(3), 0, 0).is_err());
    assert!(EnumerationValue::new(TypeId(3), 2, 2).is_err());
    assert!(EnumerationValue::new(TypeId(3), u32::MAX, 2).is_err());
}

#[test]
fn primitive_real_bits_preserve_zero_subnormal_and_finite_boundaries() {
    for value in [0.0, -0.0, f64::MIN, f64::MAX, f64::from_bits(1)] {
        let real = RealValue::new(value).unwrap();
        assert_eq!(real.to_bits(), value.to_bits());
        assert_eq!(real.get().to_bits(), value.to_bits());
        let binary = postcard::to_allocvec(&real).unwrap();
        let restored: RealValue = postcard::from_bytes(&binary).unwrap();
        let text = ron::ser::to_string_pretty(
            &restored,
            ron::ser::PrettyConfig::default().struct_names(true),
        )
        .unwrap();
        assert!(text.contains("real_bits"));
        assert_eq!(ron::from_str::<RealValue>(&text).unwrap(), real);
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(RealValue::new(value), Err(ValueError::NonFiniteReal));
        assert!(
            postcard::from_bytes::<RealValue>(&postcard::to_allocvec(&value.to_bits()).unwrap())
                .is_err()
        );
    }
}

#[test]
fn primitive_physical_units_check_scale_and_signed_range() {
    assert_eq!(
        PhysicalValue::from_unit(TypeId(2), -4, 1000)
            .unwrap()
            .primary_units,
        -4000
    );
    assert!(PhysicalValue::from_unit(TypeId(2), 1, 0).is_err());
    assert!(PhysicalValue::from_unit(TypeId(2), 1, u64::MAX).is_err());
    assert_eq!(
        PhysicalValue::from_unit(TypeId(2), i64::MAX, 2),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        PhysicalValue::from_unit(TypeId(2), i64::MIN, 1)
            .unwrap()
            .primary_units,
        i64::MIN
    );
}

#[test]
fn primitive_time_units_and_resolutions_are_exact() {
    for unit in [
        TimeUnit::Femtosecond,
        TimeUnit::Picosecond,
        TimeUnit::Nanosecond,
        TimeUnit::Microsecond,
        TimeUnit::Millisecond,
        TimeUnit::Second,
        TimeUnit::Minute,
        TimeUnit::Hour,
    ] {
        assert_eq!(
            TimeValue::from_unit(1, unit).unwrap().0,
            unit.femtoseconds()
        );
    }
    assert_eq!(
        TimeValue::from_unit(3, TimeUnit::Hour),
        Err(ValueError::Overflow)
    );
    assert_eq!(
        TimeValue(-1).to_ticks(TimeResolution::Femtosecond),
        Err(ValueError::NegativeDelay)
    );
    assert!(matches!(
        TimeValue(1).to_ticks(TimeResolution::Picosecond),
        Err(ValueError::ResolutionLoss { .. })
    ));
    for resolution in [
        TimeResolution::Femtosecond,
        TimeResolution::Picosecond,
        TimeResolution::Nanosecond,
        TimeResolution::Microsecond,
        TimeResolution::Millisecond,
        TimeResolution::Second,
    ] {
        let maximum = SimulationTime::new(resolution.max_ticks(), resolution).unwrap();
        assert!(maximum.to_time().unwrap().0 >= 0);
        assert_eq!(
            SimulationTime::new(resolution.max_ticks() + 1, resolution),
            Err(ValueError::Overflow)
        );
        assert!(
            maximum
                .checked_advance(TimeValue::from_unit(1, TimeUnit::Second).unwrap())
                .is_err()
        );
        let binary = postcard::to_allocvec(&maximum).unwrap();
        assert_eq!(
            postcard::from_bytes::<SimulationTime>(&binary).unwrap(),
            maximum
        );
        let text = ron::ser::to_string_pretty(
            &maximum,
            ron::ser::PrettyConfig::default().struct_names(true),
        )
        .unwrap();
        assert_eq!(ron::from_str::<SimulationTime>(&text).unwrap(), maximum);
    }
}

#[test]
fn primitive_fixture_round_trips_every_kind_and_signed_boundary() {
    let fixture = fixture::Fixture::load("values/primitives.ron").unwrap();
    let values = vec![
        PrimitiveValue::Boolean(false),
        PrimitiveValue::Boolean(true),
        PrimitiveValue::Bit(BitValue::Zero),
        PrimitiveValue::Bit(BitValue::One),
        PrimitiveValue::Enumeration(EnumerationValue::new(TypeId(3), 1, 2).unwrap()),
        PrimitiveValue::Integer(IntegerValue(i64::MIN)),
        PrimitiveValue::Integer(IntegerValue(i64::MAX)),
        PrimitiveValue::Real(RealValue::new(-0.0).unwrap()),
        PrimitiveValue::Real(RealValue::from_bits(1).unwrap()),
        PrimitiveValue::Physical(PhysicalValue::from_unit(TypeId(2), i64::MIN, 1).unwrap()),
        PrimitiveValue::Time(TimeValue(i64::MIN)),
        PrimitiveValue::Time(TimeValue(i64::MAX)),
    ];
    for result in fixture::round_trips(&values).unwrap() {
        fixture
            .check(&fixture::Observation {
                kind: fixture::FixtureKind::HarnessSelfTest,
                stage: osv::diagnostics::DiagnosticStage::Artifact,
                diagnostics: Vec::new(),
                exit_status: 0,
                result,
                output: String::new(),
            })
            .unwrap();
    }
}

#[test]
fn primitive_decoding_cannot_bypass_real_or_simulation_time_invariants() {
    #[derive(serde::Serialize)]
    #[serde(rename = "SimulationTime")]
    struct InvalidTime {
        ticks: u64,
        resolution: TimeResolution,
    }
    let invalid = InvalidTime {
        ticks: u64::MAX,
        resolution: TimeResolution::Femtosecond,
    };
    let binary = postcard::to_allocvec(&invalid).unwrap();
    assert!(postcard::from_bytes::<SimulationTime>(&binary).is_err());
    let text = ron::ser::to_string_pretty(
        &invalid,
        ron::ser::PrettyConfig::default().struct_names(true),
    )
    .unwrap();
    assert!(ron::from_str::<SimulationTime>(&text).is_err());
    for real_bits in [
        f64::NAN.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
    ] {
        assert!(ron::from_str::<RealValue>(&format!("RealValue(real_bits: {real_bits})")).is_err());
    }
    let value: EnumerationValue = ron::from_str("(type_id: (3), position: 99)").unwrap();
    assert!(value.validate(TypeId(3), 2).is_err());
}

#[test]
fn primitive_time_advance_keeps_zero_and_resolution_boundaries() {
    let zero = SimulationTime::new(0, TimeResolution::Nanosecond).unwrap();
    assert_eq!(zero.checked_advance(TimeValue(0)).unwrap(), zero);
    let advanced = zero
        .checked_advance(TimeValue::from_unit(2, TimeUnit::Nanosecond).unwrap())
        .unwrap();
    assert_eq!(advanced.ticks(), 2);
    assert_eq!(advanced.resolution(), TimeResolution::Nanosecond);
    assert_eq!(advanced.to_time().unwrap(), TimeValue(2_000_000));
    assert_eq!(
        zero.checked_advance(TimeValue(-1)),
        Err(ValueError::NegativeDelay)
    );
    assert!(matches!(
        zero.checked_advance(TimeValue(1)),
        Err(ValueError::ResolutionLoss { .. })
    ));
}
