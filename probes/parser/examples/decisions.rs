use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MAX_INPUT_BYTES: usize = 4096;
const MAX_OPERATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Stage {
    Analysis,
    Elaboration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Operation {
    CheckedAddI64 { left: i64, right: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Envelope {
    schema_version: u32,
    stage: Stage,
    real_bits: u64,
    #[serde(deserialize_with = "bounded_operations")]
    operations: Vec<Operation>,
}

fn bounded_operations<'de, D>(deserializer: D) -> Result<Vec<Operation>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OperationsVisitor;

    impl<'de> de::Visitor<'de> for OperationsVisitor {
        type Value = Vec<Operation>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "at most {MAX_OPERATIONS} typed operations")
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            if sequence
                .size_hint()
                .is_some_and(|size| size > MAX_OPERATIONS)
            {
                return Err(de::Error::custom("operation count exceeds probe limit"));
            }
            let mut operations = Vec::new();
            while let Some(operation) = sequence.next_element()? {
                if operations.len() == MAX_OPERATIONS {
                    return Err(de::Error::custom("operation count exceeds probe limit"));
                }
                operations.push(operation);
            }
            Ok(operations)
        }
    }

    deserializer.deserialize_seq(OperationsVisitor)
}

#[derive(Debug, PartialEq, Eq)]
enum ProbeError {
    InputTooLarge,
    Malformed,
    TrailingInput,
    WrongVersion,
    WrongStage,
}

fn validate(envelope: Envelope, expected_stage: Stage) -> Result<Envelope, ProbeError> {
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(ProbeError::WrongVersion);
    }
    if envelope.stage != expected_stage {
        return Err(ProbeError::WrongStage);
    }
    Ok(envelope)
}

fn decode_postcard(input: &[u8], expected_stage: Stage) -> Result<Envelope, ProbeError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ProbeError::InputTooLarge);
    }
    let (envelope, remaining) =
        postcard::take_from_bytes(input).map_err(|_| ProbeError::Malformed)?;
    if !remaining.is_empty() {
        return Err(ProbeError::TrailingInput);
    }
    validate(envelope, expected_stage)
}

fn decode_ron(input: &[u8], expected_stage: Stage) -> Result<Envelope, ProbeError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ProbeError::InputTooLarge);
    }
    let envelope = ron::Options::default()
        .with_recursion_limit(64)
        .from_bytes(input)
        .map_err(|_| ProbeError::Malformed)?;
    validate(envelope, expected_stage)
}

fn sample(stage: Stage) -> Envelope {
    Envelope {
        schema_version: SCHEMA_VERSION,
        stage,
        real_bits: (-0.0_f64).to_bits(),
        operations: vec![Operation::CheckedAddI64 { left: 40, right: 2 }],
    }
}

type Kernel = fn(i64, i64) -> Option<i64>;

struct BoundOperation {
    kernel: Kernel,
    left: i64,
    right: i64,
}

fn bind(operation: &Operation) -> BoundOperation {
    match *operation {
        Operation::CheckedAddI64 { left, right } => BoundOperation {
            kernel: i64::checked_add,
            left,
            right,
        },
    }
}

fn main() {
    let original = sample(Stage::Elaboration);
    let binary = postcard::to_allocvec(&original).unwrap();
    let ron_text = ron::ser::to_string_pretty(&original, Default::default()).unwrap();
    for decoded in [
        decode_postcard(&binary, Stage::Elaboration).unwrap(),
        decode_ron(ron_text.as_bytes(), Stage::Elaboration).unwrap(),
    ] {
        let bound = bind(&decoded.operations[0]);
        assert_eq!((bound.kernel)(bound.left, bound.right), Some(42));
    }
    println!(
        "synthetic envelope: Postcard {} bytes; rebound checked-add = 42\n{ron_text}",
        binary.len(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_typed_dispatch_roundtrip() {
        for stage in [Stage::Analysis, Stage::Elaboration] {
            let original = sample(stage);
            let postcard_bytes = postcard::to_allocvec(&original).unwrap();
            let from_postcard = decode_postcard(&postcard_bytes, stage).unwrap();
            let ron_text = ron::ser::to_string_pretty(&from_postcard, Default::default()).unwrap();
            let from_ron = decode_ron(ron_text.as_bytes(), stage).unwrap();

            for decoded in [from_postcard, from_ron] {
                assert_eq!(decoded, original);
                let bound = bind(&decoded.operations[0]);
                assert_eq!((bound.kernel)(bound.left, bound.right), Some(42));
                assert_eq!(postcard::to_allocvec(&decoded).unwrap(), postcard_bytes);
                assert_eq!(
                    ron::ser::to_string_pretty(&decoded, Default::default()).unwrap(),
                    ron_text
                );
            }
            assert!(ron_text.contains("CheckedAddI64"));
            assert!(ron_text.contains("real_bits"));
        }
    }

    #[test]
    fn decisions_checked_integer_and_real_bit_boundaries() {
        for (left, right, expected) in [
            (i64::MIN, 0, Some(i64::MIN)),
            (i64::MAX, 0, Some(i64::MAX)),
            (i64::MAX, 1, None),
            (i64::MIN, -1, None),
        ] {
            for real_bits in [0, (-0.0_f64).to_bits(), f64::MAX.to_bits(), 1] {
                let mut original = sample(Stage::Elaboration);
                original.operations = vec![Operation::CheckedAddI64 { left, right }];
                original.real_bits = real_bits;
                let binary = postcard::to_allocvec(&original).unwrap();
                let text = ron::ser::to_string_pretty(&original, Default::default()).unwrap();
                for decoded in [
                    decode_postcard(&binary, Stage::Elaboration).unwrap(),
                    decode_ron(text.as_bytes(), Stage::Elaboration).unwrap(),
                ] {
                    assert_eq!(decoded, original);
                    let bound = bind(&decoded.operations[0]);
                    assert_eq!((bound.kernel)(bound.left, bound.right), expected);
                }
            }
        }
    }

    #[test]
    fn decisions_rejects_oversized_input_and_sequences_in_both_formats() {
        let oversized = vec![0; MAX_INPUT_BYTES + 1];
        assert_eq!(
            decode_postcard(&oversized, Stage::Analysis),
            Err(ProbeError::InputTooLarge)
        );
        assert_eq!(
            decode_ron(&oversized, Stage::Analysis),
            Err(ProbeError::InputTooLarge)
        );
        for count in [0, MAX_OPERATIONS, MAX_OPERATIONS + 1] {
            let mut original = sample(Stage::Elaboration);
            original.operations = vec![original.operations[0].clone(); count];
            let binary = postcard::to_allocvec(&original).unwrap();
            let text = ron::ser::to_string(&original).unwrap();
            assert_eq!(
                decode_postcard(&binary, Stage::Elaboration).is_ok(),
                count <= MAX_OPERATIONS
            );
            assert_eq!(
                decode_ron(text.as_bytes(), Stage::Elaboration).is_ok(),
                count <= MAX_OPERATIONS
            );
        }
    }

    #[test]
    fn decisions_rejects_claimed_length_before_reading_elements() {
        #[derive(Debug, Deserialize)]
        struct LimitedOperations(#[serde(deserialize_with = "bounded_operations")] Vec<Operation>);

        for claimed_length in [MAX_OPERATIONS as u64 + 1, u64::MAX] {
            let prefix_only = postcard::to_allocvec(&claimed_length).unwrap();
            assert!(postcard::from_bytes::<LimitedOperations>(&prefix_only).is_err());
        }
        let empty = postcard::from_bytes::<LimitedOperations>(&[0]).unwrap();
        assert!(empty.0.is_empty());
    }

    #[test]
    fn decisions_rejects_truncation_trailing_input_and_unknown_operations() {
        let original = sample(Stage::Elaboration);
        let binary = postcard::to_allocvec(&original).unwrap();
        for length in 0..binary.len() {
            assert!(decode_postcard(&binary[..length], Stage::Elaboration).is_err());
        }
        let mut trailing = binary;
        trailing.push(0);
        assert!(postcard::from_bytes::<Envelope>(&trailing).is_ok());
        assert_eq!(
            decode_postcard(&trailing, Stage::Elaboration),
            Err(ProbeError::TrailingInput)
        );
        let text = ron::ser::to_string(&original).unwrap();
        assert!(decode_ron(format!("{text} junk").as_bytes(), Stage::Elaboration).is_err());
        assert!(decode_ron(&text.as_bytes()[..text.len() - 1], Stage::Elaboration).is_err());
        let unknown = postcard::to_allocvec(&999_u32).unwrap();
        assert!(postcard::from_bytes::<Operation>(&unknown).is_err());
        assert!(decode_ron(
            text.replace("CheckedAddI64", "FutureOperation").as_bytes(),
            Stage::Elaboration
        )
        .is_err());
    }

    #[test]
    fn decisions_rejects_incompatible_version_and_stage_in_both_formats() {
        let mut original = sample(Stage::Analysis);
        for (version, expected_stage, expected_error) in [
            (
                SCHEMA_VERSION + 1,
                Stage::Analysis,
                ProbeError::WrongVersion,
            ),
            (SCHEMA_VERSION, Stage::Elaboration, ProbeError::WrongStage),
        ] {
            original.schema_version = version;
            let binary = postcard::to_allocvec(&original).unwrap();
            let text = ron::ser::to_string(&original).unwrap();
            assert_eq!(
                decode_postcard(&binary, expected_stage).unwrap_err(),
                expected_error
            );
            assert_eq!(
                decode_ron(text.as_bytes(), expected_stage).unwrap_err(),
                expected_error
            );
        }
    }

    #[test]
    fn decisions_ron_recursion_is_bounded() {
        let nested = format!("{}0{}", "[".repeat(80), "]".repeat(80));
        let error = ron::Options::default()
            .with_recursion_limit(64)
            .from_str::<ron::Value>(&nested)
            .unwrap_err();
        assert_eq!(error.code, ron::error::Error::ExceededRecursionLimit);
    }
}
