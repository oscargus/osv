#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;

use num_bigint::{BigInt, BigUint};
use osv::values::*;
use proptest::prelude::*;

#[test]
fn packed_binary_zero_and_full_word_masks_do_not_shift_out_of_range() {
    let empty = PackedBinary::masked_from_u64(0, u64::MAX).unwrap();
    assert_eq!(empty.width(), 0);
    assert_eq!(empty.storage(), &PackedStorage::Small(0));
    let full = PackedBinary::masked_from_u64(64, u64::MAX).unwrap();
    assert_eq!(full.width(), 64);
    assert_eq!(full.storage(), &PackedStorage::Small(u64::MAX));
    assert_eq!(
        PackedBinary::masked_from_u64(63, u64::MAX)
            .unwrap()
            .storage(),
        &PackedStorage::Small(i64::MAX.unsigned_abs())
    );
}

#[test]
fn packed_binary_storage_thresholds_preserve_width_and_leading_zeroes() {
    for width in [0, 1, 31, 32, 63, 64, 65, 128, 4096, 131_072] {
        let value = PackedBinary::from_u64(width, 0).unwrap();
        assert_eq!(value.width(), width);
        assert_eq!(value.unsigned_value(), BigUint::from(0u8));
        assert_eq!(
            matches!(value.storage(), PackedStorage::Small(_)),
            width <= 64
        );
        if width != 0 {
            let mut value = value;
            value.set_bit(width - 1, true).unwrap();
            assert!(value.bit(width - 1).unwrap());
            assert_eq!(value.unsigned_value().bits(), width);
        }
    }
    assert!(PackedBinary::from_u64(0, 1).is_err());
    assert!(PackedBinary::from_u64(3, 8).is_err());
    assert!(PackedBinary::from_u64(MAX_VECTOR_BITS + 1, 0).is_err());
    assert!(PackedBinary::from_u64(0, 0).unwrap().bit(0).is_err());
}

#[test]
fn packed_binary_signed_interpretation_is_not_a_storage_flag() {
    for width in [1u64, 31, 32, 63, 64, 65, 128] {
        let shift = usize::try_from(width).unwrap();
        let all_ones = (BigUint::from(1u8) << shift) - BigUint::from(1u8);
        let value = PackedBinary::from_biguint(width, all_ones.clone()).unwrap();
        assert_eq!(value.unsigned_value(), all_ones);
        assert_eq!(value.signed_value().unwrap(), BigInt::from(-1));
        let sign_only =
            PackedBinary::from_biguint(width, BigUint::from(1u8) << (shift - 1)).unwrap();
        assert_eq!(
            sign_only.signed_value().unwrap(),
            -(BigInt::from(1u8) << (shift - 1))
        );
    }
    assert_eq!(
        PackedBinary::from_u64(0, 0)
            .unwrap()
            .signed_value()
            .unwrap(),
        BigInt::from(0)
    );
}

#[test]
fn packed_binary_logical_indexing_preserves_both_directions() {
    let value = PackedBinary::from_u64(4, 0b1010).unwrap();
    for range in [
        IndexRange {
            left: 3,
            right: 6,
            direction: Direction::Ascending,
        },
        IndexRange {
            left: 6,
            right: 3,
            direction: Direction::Descending,
        },
    ] {
        for offset in 0..4 {
            assert_eq!(
                value
                    .logical_bit(range, range.index_at(offset).unwrap())
                    .unwrap(),
                offset % 2 == 0
            );
        }
    }
    assert!(
        value
            .logical_bit(
                IndexRange {
                    left: 0,
                    right: 2,
                    direction: Direction::Ascending
                },
                0
            )
            .is_err()
    );
}

#[test]
fn packed_binary_encoding_has_canonical_little_endian_limbs() {
    let value =
        PackedBinary::from_biguint(128, (BigUint::from(1u8) << 64usize) + BigUint::from(2u8))
            .unwrap();
    let fixture = fixture::Fixture::load("values/binary.ron").unwrap();
    for result in fixture::round_trips(&value).unwrap() {
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
    for invalid in [
        "PackedBinary(width: 65, storage: Small(1))",
        "PackedBinary(width: 64, storage: Big(limbs_le: [1]))",
        "PackedBinary(width: 65, storage: Big(limbs_le: [1, 0]))",
        "PackedBinary(width: 1, storage: Small(2))",
        "PackedBinary(width: 65, storage: Big(limbs_le: [0, 0, 2]))",
    ] {
        assert!(ron::from_str::<PackedBinary>(invalid).is_err(), "{invalid}");
    }
}

proptest! {
    #[test]
    fn packed_binary_randomized_storage_bits_and_codecs(
        width in prop::sample::select(vec![0u64, 1, 31, 32, 63, 64, 65, 128, 4096]),
        limbs in prop::collection::vec(any::<u32>(), 0..132),
    ) {
        let original = BigUint::new(limbs);
        let value = PackedBinary::masked_from_biguint(width, original.clone()).unwrap();
        prop_assert_eq!(value.width(), width);
        for bit in 0..width { prop_assert_eq!(value.bit(bit).unwrap(), original.bit(bit)); }
        for restored in fixture::round_trips(&value).unwrap() { prop_assert_eq!(restored, value.clone()); }
    }
}
