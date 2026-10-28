#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/artifact.rs"]
mod support;

use std::collections::BTreeMap;

use osv::{
    analysis::{
        constant::{self, FoldResult},
        expressions::*,
    },
    artifacts::*,
    intrinsics::*,
    source::{ByteSpan, SourceId},
    values::*,
};

#[test]
fn expression_constant_folding_checks_identity_and_never_observes_runtime_time() {
    let registry = IntrinsicRegistry::standard();
    let reference = |id| {
        registry
            .resolve(&registry.declaration(id).unwrap().unwrap())
            .unwrap()
            .unwrap()
    };
    assert_eq!(
        constant::fold(
            &reference(IntrinsicId::IntegerAdd),
            &[
                PrimitiveValue::Integer(IntegerValue(2)),
                PrimitiveValue::Integer(IntegerValue(3))
            ]
        )
        .unwrap(),
        FoldResult::Value(PrimitiveValue::Integer(IntegerValue(5)))
    );
    assert_eq!(
        constant::fold(&reference(IntrinsicId::Now), &[]).unwrap(),
        FoldResult::NotYetStatic
    );
}

fn reference(id: IntrinsicId) -> IntrinsicReference {
    let registry = IntrinsicRegistry::standard();
    registry
        .resolve(&registry.declaration(id).unwrap().unwrap())
        .unwrap()
        .unwrap()
}

fn plan() -> (ArtifactEnvelope, ExpressionPlan) {
    let mut artifact = support::sample();
    let native = install_standard(&mut artifact).unwrap();
    let type_id = native.types[&NativeType::Integer];
    let span = ByteSpan {
        source: SourceId(4),
        start: 0,
        end: 1,
    };
    let nodes = vec![
        TypedExpression {
            id: ExpressionId(0),
            type_id,
            span,
            kind: ExpressionKind::Literal(PrimitiveValue::Integer(IntegerValue(2))),
        },
        TypedExpression {
            id: ExpressionId(1),
            type_id,
            span,
            kind: ExpressionKind::Literal(PrimitiveValue::Integer(IntegerValue(3))),
        },
        TypedExpression {
            id: ExpressionId(2),
            type_id,
            span,
            kind: ExpressionKind::Intrinsic {
                reference: Box::new(reference(IntrinsicId::IntegerAdd)),
                arguments: vec![ExpressionId(0), ExpressionId(1)],
            },
        },
    ];
    (
        artifact,
        ExpressionPlan {
            nodes,
            result: ExpressionId(2),
        },
    )
}

#[test]
fn expression_plan_round_trips_resolved_types_and_intrinsics_and_matches_folded_result() {
    let (artifact, plan) = plan();
    let expected = FoldResult::Value(PrimitiveValue::Integer(IntegerValue(5)));
    assert_eq!(
        plan.evaluate(&artifact, &BTreeMap::new()).unwrap(),
        expected
    );
    for restored in fixture::round_trips(&plan).unwrap() {
        restored.validate(&artifact).unwrap();
        assert_eq!(
            restored.evaluate(&artifact, &BTreeMap::new()).unwrap(),
            expected
        );
    }
    let mut folded = plan;
    folded.nodes = vec![TypedExpression {
        kind: ExpressionKind::Literal(PrimitiveValue::Integer(IntegerValue(5))),
        ..folded.nodes[2].clone()
    }];
    assert_eq!(
        folded.evaluate(&artifact, &BTreeMap::new()).unwrap(),
        expected
    );
}

#[test]
fn expression_plan_rejects_duplicate_dangling_forward_and_wrong_type_references() {
    for change in 0..5 {
        let (artifact, mut plan) = plan();
        match change {
            0 => plan.nodes[1].id = ExpressionId(0),
            1 => plan.result = ExpressionId(99),
            2 => {
                if let ExpressionKind::Intrinsic { arguments, .. } = &mut plan.nodes[2].kind {
                    arguments[0] = ExpressionId(99);
                }
            }
            3 => plan.nodes[0].kind = ExpressionKind::Literal(PrimitiveValue::Boolean(true)),
            4 => plan.nodes[0].type_id = TypeId(99),
            _ => unreachable!(),
        }
        assert!(plan.validate(&artifact).is_err());
    }
}

#[test]
fn expression_unavailable_constants_and_signal_reads_stay_nonstatic() {
    let (mut artifact, mut plan) = plan();
    let type_id = plan.nodes[0].type_id;
    artifact.symbols[0].type_id = Some(type_id);
    plan.nodes[0].kind = ExpressionKind::Name(SymbolId(5));
    let values = BTreeMap::from([(SymbolId(5), PrimitiveValue::Integer(IntegerValue(2)))]);
    assert_eq!(
        plan.evaluate(&artifact, &values).unwrap(),
        FoldResult::NotYetStatic
    );
    artifact.symbols[0].kind = SymbolKind::Constant;
    assert_eq!(
        plan.evaluate(&artifact, &BTreeMap::new()).unwrap(),
        FoldResult::NotYetStatic
    );
    assert_eq!(
        plan.evaluate(&artifact, &values).unwrap(),
        FoldResult::Value(PrimitiveValue::Integer(IntegerValue(5)))
    );
}

#[test]
fn expression_integer_time_and_boolean_kernels_are_checked() {
    let integer = |value| PrimitiveValue::Integer(IntegerValue(value));
    for (id, inputs, expected) in [
        (
            IntrinsicId::IntegerSubtract,
            vec![integer(7), integer(3)],
            integer(4),
        ),
        (
            IntrinsicId::IntegerMultiply,
            vec![integer(7), integer(3)],
            integer(21),
        ),
        (
            IntrinsicId::IntegerDivide,
            vec![integer(-7), integer(3)],
            integer(-2),
        ),
        (
            IntrinsicId::IntegerLess,
            vec![integer(-7), integer(3)],
            PrimitiveValue::Boolean(true),
        ),
        (
            IntrinsicId::BooleanAnd,
            vec![
                PrimitiveValue::Boolean(true),
                PrimitiveValue::Boolean(false),
            ],
            PrimitiveValue::Boolean(false),
        ),
        (
            IntrinsicId::TimeAdd,
            vec![
                PrimitiveValue::Time(TimeValue(1)),
                PrimitiveValue::Time(TimeValue(2)),
            ],
            PrimitiveValue::Time(TimeValue(3)),
        ),
    ] {
        assert_eq!(
            constant::fold(&reference(id), &inputs).unwrap(),
            FoldResult::Value(expected)
        );
    }
    for (id, inputs) in [
        (IntrinsicId::IntegerAdd, vec![integer(i64::MAX), integer(1)]),
        (IntrinsicId::IntegerDivide, vec![integer(1), integer(0)]),
        (IntrinsicId::IntegerNegate, vec![integer(i64::MIN)]),
    ] {
        assert!(constant::fold(&reference(id), &inputs).is_err());
    }
    assert!(
        constant::fold(
            &reference(IntrinsicId::IntegerAdd),
            &[PrimitiveValue::Boolean(true), integer(1)]
        )
        .is_err()
    );
    assert!(constant::fold(&reference(IntrinsicId::IntegerAdd), &[]).is_err());
}
