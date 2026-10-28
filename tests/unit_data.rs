#[path = "support/artifact.rs"]
mod support;

use osv::{
    analysis::{
        ObjectDeclaration, ObjectInitialization, PendingProcess,
        expressions::{ExpressionId, ExpressionKind, ExpressionPlan, TypedExpression},
    },
    artifacts::{
        store::{LibraryStore, StoreError, StoreOptions},
        *,
    },
    intrinsics::{NativeType, install_standard},
    source::{ByteSpan, SourceId},
    values::PrimitiveValue,
};

#[test]
fn analyzed_object_expression_round_trips_without_recursive_validation() {
    let mut artifact = support::sample();
    let prelude = install_standard(&mut artifact).unwrap();
    let type_id = prelude.types[&NativeType::Boolean];
    artifact.symbols[0].type_id = Some(type_id);
    let initialization = ExpressionPlan {
        nodes: vec![TypedExpression {
            id: ExpressionId(0),
            type_id,
            span: ByteSpan {
                source: SourceId(4),
                start: 0,
                end: 1,
            },
            kind: ExpressionKind::Literal(PrimitiveValue::Boolean(true)),
        }],
        result: ExpressionId(0),
    };
    let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
        unreachable!()
    };
    analysis.objects.push(ObjectDeclaration {
        symbol: SymbolId(5),
        initialization: ObjectInitialization::Expression(initialization),
    });
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let loaded = decode(
            &encode(&artifact, format).unwrap(),
            None,
            DecodeLimits::default(),
        )
        .unwrap();
        assert_eq!(loaded, artifact.canonicalized().unwrap());
    }
}

fn context() -> ArtifactEnvelope {
    let mut artifact = support::sample();
    let prelude = install_standard(&mut artifact).unwrap();
    artifact.symbols[0].type_id = Some(prelude.types[&NativeType::Boolean]);
    artifact
}

#[test]
fn analyzed_default_objects_and_pending_processes_preserve_exact_records() {
    let mut artifact = context();
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(99),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
        unreachable!()
    };
    analysis.objects.push(ObjectDeclaration {
        symbol: SymbolId(5),
        initialization: ObjectInitialization::Default(PrimitiveValue::Boolean(false)),
    });
    analysis.pending_processes.push(PendingProcess {
        name: Some(Identifier::Basic("worker".into())),
        scope: ScopeId(99),
        span: ByteSpan {
            source: SourceId(4),
            start: 0,
            end: 1,
        },
    });
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let loaded = decode(
            &encode(&artifact, format).unwrap(),
            None,
            DecodeLimits::default(),
        )
        .unwrap();
        assert_eq!(loaded, artifact.canonicalized().unwrap());
        assert_eq!(
            loaded.fingerprint().unwrap(),
            artifact.fingerprint().unwrap()
        );
    }
}

#[test]
fn analyzed_unit_data_rejects_bad_object_and_process_references() {
    for change in 0..4 {
        let mut artifact = context();
        let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
            unreachable!()
        };
        analysis.objects.push(ObjectDeclaration {
            symbol: if change == 0 {
                SymbolId(999)
            } else {
                SymbolId(5)
            },
            initialization: ObjectInitialization::Default(PrimitiveValue::Boolean(false)),
        });
        match change {
            1 => analysis.objects.push(analysis.objects[0].clone()),
            2 => {
                analysis.objects[0].initialization = ObjectInitialization::Default(
                    PrimitiveValue::Integer(osv::values::IntegerValue(1)),
                )
            }
            3 => analysis.pending_processes.push(PendingProcess {
                name: None,
                scope: ScopeId(1),
                span: ByteSpan {
                    source: SourceId(4),
                    start: 0,
                    end: 1,
                },
            }),
            _ => {}
        }
        assert!(artifact.validate(artifact.stage()).is_err());
    }
}

#[test]
fn analyzed_pending_processes_cannot_be_published_as_successful_units() {
    let mut artifact = context();
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(99),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
        unreachable!()
    };
    analysis.pending_processes.push(PendingProcess {
        name: None,
        scope: ScopeId(99),
        span: ByteSpan {
            source: SourceId(4),
            start: 0,
            end: 1,
        },
    });
    let directory = tempfile::tempdir().unwrap();
    let mut store = LibraryStore::open(
        directory.path(),
        Identifier::Basic("work".into()),
        StoreOptions::default(),
    )
    .unwrap();
    assert!(matches!(
        store.publish(&artifact, ArtifactFormat::Binary),
        Err(StoreError::IncompleteAnalysis(_))
    ));
    assert!(store.catalog().entries.is_empty());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn analyzed_unit_data_rejects_malformed_embedded_plans_and_process_sources() {
    for change in 0..4 {
        let mut artifact = context();
        let type_id = artifact.symbols[0].type_id.unwrap();
        let expression = ExpressionPlan {
            nodes: vec![TypedExpression {
                id: ExpressionId(0),
                type_id,
                span: ByteSpan {
                    source: SourceId(4),
                    start: 0,
                    end: 1,
                },
                kind: ExpressionKind::Literal(PrimitiveValue::Boolean(true)),
            }],
            result: ExpressionId(0),
        };
        let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
            unreachable!()
        };
        analysis.objects.push(ObjectDeclaration {
            symbol: SymbolId(5),
            initialization: ObjectInitialization::Expression(expression),
        });
        match change {
            0 => {
                if let ObjectInitialization::Expression(plan) =
                    &mut analysis.objects[0].initialization
                {
                    plan.result = ExpressionId(99);
                }
            }
            1 => {
                if let ObjectInitialization::Expression(plan) =
                    &mut analysis.objects[0].initialization
                {
                    plan.nodes[0].kind = ExpressionKind::Name(SymbolId(999));
                }
            }
            2 => analysis.pending_processes.push(PendingProcess {
                name: None,
                scope: ScopeId(1),
                span: ByteSpan {
                    source: SourceId(999),
                    start: 0,
                    end: 1,
                },
            }),
            3 => {
                analysis.objects[0].initialization =
                    ObjectInitialization::Default(PrimitiveValue::Boolean(false));
                artifact.symbols[0].kind = SymbolKind::Constant;
            }
            _ => unreachable!(),
        }
        assert!(artifact.validate(artifact.stage()).is_err());
    }
}
