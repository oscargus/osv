#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;

use osv::{artifacts::TypeId, values::*};

#[test]
fn array_null_ranges_have_zero_length_before_endpoint_arithmetic() {
    for range in [
        IndexRange {
            left: 9,
            right: 7,
            direction: Direction::Ascending,
        },
        IndexRange {
            left: 7,
            right: 9,
            direction: Direction::Descending,
        },
        IndexRange {
            left: i64::MAX,
            right: i64::MIN,
            direction: Direction::Ascending,
        },
        IndexRange {
            left: i64::MIN,
            right: i64::MAX,
            direction: Direction::Descending,
        },
    ] {
        assert!(range.is_null());
        assert_eq!(range.length(), Ok(0));
    }
    assert_eq!(
        IndexRange {
            left: i64::MIN,
            right: i64::MAX,
            direction: Direction::Ascending
        }
        .length(),
        Err(ValueError::Overflow)
    );
}

fn dimension(left: i64, right: i64, direction: Direction) -> ArrayDimension {
    ArrayDimension {
        index_type: TypeId(2),
        range: RangeConstraint::Constrained(IndexRange {
            left,
            right,
            direction,
        }),
    }
}

#[test]
fn array_logical_indices_map_from_declared_left_in_both_directions() {
    for (range, expected) in [
        (
            IndexRange {
                left: 7,
                right: 9,
                direction: Direction::Ascending,
            },
            vec![7, 8, 9],
        ),
        (
            IndexRange {
                left: -3,
                right: -5,
                direction: Direction::Descending,
            },
            vec![-3, -4, -5],
        ),
    ] {
        assert_eq!(range.length().unwrap(), 3);
        for (offset, index) in expected.into_iter().enumerate() {
            let offset = u64::try_from(offset).unwrap();
            assert_eq!(range.offset(index).unwrap(), offset);
            assert_eq!(range.index_at(offset).unwrap(), index);
        }
        assert!(range.index_at(3).is_err());
        assert!(range.offset(i64::MAX).is_err());
        assert!(range.offset(i64::MIN).is_err());
    }
}

#[test]
fn array_extreme_singletons_and_large_offsets_do_not_wrap() {
    for endpoint in [i64::MIN, i64::MAX] {
        for direction in [Direction::Ascending, Direction::Descending] {
            let range = IndexRange {
                left: endpoint,
                right: endpoint,
                direction,
            };
            assert_eq!(range.length(), Ok(1));
            assert_eq!(range.offset(endpoint), Ok(0));
            assert_eq!(range.index_at(0), Ok(endpoint));
        }
    }
    let range = IndexRange {
        left: i64::MIN,
        right: i64::MAX - 1,
        direction: Direction::Ascending,
    };
    assert_eq!(range.length(), Ok(u64::MAX));
    assert_eq!(range.index_at(u64::MAX - 1), Ok(i64::MAX - 1));
    assert_eq!(range.offset(i64::MAX - 1), Ok(u64::MAX - 1));
    assert!(range.index_at(u64::MAX).is_err());
}

#[test]
fn array_multidimensional_shape_uses_rightmost_dimension_fastest() {
    let shape = ArrayShape {
        dimensions: vec![
            dimension(7, 8, Direction::Ascending),
            dimension(-1, -3, Direction::Descending),
        ],
    };
    assert_eq!(shape.element_count(), Ok(6));
    for (offset, indices) in [[7, -1], [7, -2], [7, -3], [8, -1], [8, -2], [8, -3]]
        .into_iter()
        .enumerate()
    {
        let offset = u64::try_from(offset).unwrap();
        assert_eq!(shape.storage_offset(&indices), Ok(offset));
        assert_eq!(shape.indices_at(offset).unwrap(), indices);
    }
    assert!(shape.storage_offset(&[7]).is_err());
    assert!(shape.storage_offset(&[9, -1]).is_err());
    assert!(shape.indices_at(6).is_err());
}

#[test]
fn array_null_shape_has_no_elements_even_if_another_range_is_huge() {
    let shape = ArrayShape {
        dimensions: vec![
            dimension(i64::MIN, i64::MAX, Direction::Ascending),
            dimension(1, 0, Direction::Ascending),
        ],
    };
    assert_eq!(shape.element_count(), Ok(0));
    assert!(shape.storage_offset(&[0, 1]).is_err());
    assert!(shape.indices_at(0).is_err());
    let overflowing = ArrayShape {
        dimensions: vec![
            dimension(i64::MIN, i64::MAX - 1, Direction::Ascending),
            dimension(0, 1, Direction::Ascending),
        ],
    };
    assert_eq!(overflowing.element_count(), Err(ValueError::Overflow));
}

#[test]
fn array_unconstrained_and_zero_dimensional_shapes_are_explicit_errors() {
    let shape = ArrayShape {
        dimensions: vec![ArrayDimension {
            index_type: TypeId(2),
            range: RangeConstraint::Unconstrained,
        }],
    };
    assert_eq!(shape.element_count(), Err(ValueError::UnconstrainedArray));
    assert_eq!(
        shape.storage_offset(&[0]),
        Err(ValueError::UnconstrainedArray)
    );
    assert_eq!(shape.indices_at(0), Err(ValueError::UnconstrainedArray));
    assert!(
        ArrayShape {
            dimensions: Vec::new()
        }
        .element_count()
        .is_err()
    );
}

#[test]
fn array_shape_fixture_preserves_bounds_types_direction_and_codecs() {
    let fixture = fixture::Fixture::load("values/shape.ron").unwrap();
    let shape = ArrayShape {
        dimensions: vec![
            dimension(7, 8, Direction::Ascending),
            dimension(-1, -3, Direction::Descending),
        ],
    };
    for result in fixture::round_trips(&shape).unwrap() {
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
