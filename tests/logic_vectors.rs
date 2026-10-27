#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;

use osv::values::*;
use proptest::prelude::*;

const STATES: [LogicState; 9] = [
    LogicState::U,
    LogicState::X,
    LogicState::Zero,
    LogicState::One,
    LogicState::Z,
    LogicState::W,
    LogicState::L,
    LogicState::H,
    LogicState::DontCare,
];

#[test]
fn logic_only_exact_zero_and_one_are_packable() {
    for state in [
        LogicState::U,
        LogicState::X,
        LogicState::Zero,
        LogicState::One,
        LogicState::Z,
        LogicState::W,
        LogicState::L,
        LogicState::H,
        LogicState::DontCare,
    ] {
        assert_eq!(
            state.binary().is_some(),
            matches!(state, LogicState::Zero | LogicState::One)
        );
    }
    assert_eq!(LogicState::Zero.binary(), Some(false));
    assert_eq!(LogicState::One.binary(), Some(true));
}

#[test]
fn logic_fallback_packs_two_lossless_states_per_byte() {
    let value = LogicVector::from_states(&STATES).unwrap();
    assert_eq!(value.width(), 9);
    assert_eq!(value.states().unwrap(), STATES);
    assert_eq!(
        STATES
            .map(LogicState::symbol)
            .into_iter()
            .collect::<String>(),
        "UX01ZWLH-"
    );
    match value.storage() {
        LogicStorage::States { nibbles, nonbinary } => {
            assert_eq!(nibbles.len(), 5);
            assert_eq!(*nonbinary, 7);
        }
        _ => panic!("nine-state value must not become binary"),
    }
}

#[test]
fn logic_promotion_and_repacking_preserve_all_bits_at_storage_boundaries() {
    for width in [1u64, 31, 32, 63, 64, 65, 128, 4096] {
        let original = PackedBinary::masked_from_u64(width, 0xaaaa_aaaa_aaaa_aaaa).unwrap();
        let mut value = LogicVector::from_packed(original.clone());
        value.promote().unwrap();
        assert_eq!(value, LogicVector::from_packed(original.clone()));
        assert!(value.try_repack().unwrap());
        let last = value.state(width - 1).unwrap();
        for state in [LogicState::X, LogicState::L, LogicState::H, LogicState::Z] {
            value.set_state(width - 1, state).unwrap();
            assert!(!value.try_repack().unwrap());
            assert_eq!(value.state(width - 1).unwrap(), state);
        }
        value.set_state(width - 1, last).unwrap();
        assert_eq!(value, LogicVector::from_packed(original));
        assert!(matches!(value.storage(), LogicStorage::Binary(_)));
    }
}

#[test]
fn logic_null_slices_and_logical_directions_keep_storage_semantics() {
    let empty = LogicVector::from_states(&[]).unwrap();
    assert_eq!(empty.width(), 0);
    assert!(matches!(empty.storage(), LogicStorage::Binary(_)));
    assert!(empty.state(0).is_err());
    let value = LogicVector::from_states(&STATES).unwrap();
    assert_eq!(
        value.slice(2, 2).unwrap().states().unwrap(),
        [LogicState::Zero, LogicState::One]
    );
    assert!(matches!(
        value.slice(2, 2).unwrap().storage(),
        LogicStorage::Binary(_)
    ));
    assert_eq!(
        value.slice(6, 2).unwrap().states().unwrap(),
        [LogicState::L, LogicState::H]
    );
    assert_eq!(value.slice(9, 0).unwrap().width(), 0);
    assert!(value.slice(9, 1).is_err());
    assert!(value.slice(u64::MAX, 1).is_err());
    for range in [
        IndexRange {
            left: 3,
            right: 11,
            direction: Direction::Ascending,
        },
        IndexRange {
            left: 11,
            right: 3,
            direction: Direction::Descending,
        },
    ] {
        for offset in 0..9 {
            assert_eq!(
                value
                    .logical_state(range, range.index_at(offset).unwrap())
                    .unwrap(),
                STATES[usize::try_from(offset).unwrap()]
            );
        }
    }
}

#[test]
fn logic_codec_fixture_retains_named_states_and_canonical_binary_encoding() {
    let value = LogicVector::from_states(&STATES).unwrap();
    let fixture = fixture::Fixture::load("values/logic.ron").unwrap();
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
    let packed = LogicVector::from_packed(PackedBinary::from_u64(65, 1).unwrap());
    let mut promoted = packed.clone();
    promoted.promote().unwrap();
    assert_eq!(
        postcard::to_allocvec(&promoted).unwrap(),
        postcard::to_allocvec(&packed).unwrap()
    );
    assert_eq!(
        ron::to_string(&promoted).unwrap(),
        ron::to_string(&packed).unwrap()
    );
    assert!(ron::from_str::<LogicVector>("States([NotAState])").is_err());
}

proptest! {
    #[test]
    fn logic_randomized_updates_match_lossless_state_reference(
        width in prop::sample::select(vec![0usize, 1, 31, 32, 63, 64, 65, 128]),
        choices in prop::collection::vec(0usize..9, 0..100),
    ) {
        let mut reference = vec![LogicState::Zero; width];
        let mut value = LogicVector::from_states(&reference).unwrap();
        for (step, choice) in choices.into_iter().enumerate() {
            if width == 0 { break; }
            let offset = step % width;
            reference[offset] = STATES[choice];
            value.set_state(u64::try_from(offset).unwrap(), STATES[choice]).unwrap();
            prop_assert_eq!(value.states().unwrap(), reference.clone());
            prop_assert_eq!(value.try_repack().unwrap(), reference.iter().all(|state| state.binary().is_some()));
        }
        for restored in fixture::round_trips(&value).unwrap() { prop_assert_eq!(restored, value.clone()); }
    }
}
