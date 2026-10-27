#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;

use osv::{
    artifacts::{ContentFingerprint, Identifier},
    intrinsics::*,
};

#[test]
fn native_binding_uses_exact_declaration_identity_not_operator_spelling() {
    let registry = IntrinsicRegistry::standard();
    let declaration = registry
        .declaration(IntrinsicId::IntegerAdd)
        .unwrap()
        .unwrap();
    assert_eq!(
        registry.resolve(&declaration).unwrap().unwrap().id,
        IntrinsicId::IntegerAdd
    );
    let mut user = declaration.clone();
    user.unit.library = Identifier::Basic("work".into());
    assert!(registry.resolve(&user).unwrap().is_none());
    let mut stale = declaration;
    stale.abi_version += 1;
    assert!(registry.resolve(&stale).unwrap().is_none());
}

#[test]
fn native_references_and_profiles_round_trip_without_host_bindings() {
    let registry = IntrinsicRegistry::standard();
    for definition in registry.definitions() {
        let declaration = registry.declaration(definition.id).unwrap().unwrap();
        let reference = registry.resolve(&declaration).unwrap().unwrap();
        for restored in fixture::round_trips(&reference).unwrap() {
            assert_eq!(registry.definition(&restored).unwrap(), Some(definition));
        }
        let mut wrong = reference;
        wrong.id = if definition.id == IntrinsicId::IntegerAdd {
            IntrinsicId::BooleanNot
        } else {
            IntrinsicId::IntegerAdd
        };
        assert!(registry.definition(&wrong).unwrap().is_none());
    }
    for restored in fixture::round_trips(&registry.types().to_vec()).unwrap() {
        assert_eq!(restored, registry.types());
    }
}

#[test]
fn native_effects_and_time_observation_are_never_static_folding() {
    let registry = IntrinsicRegistry::standard();
    for definition in registry.definitions() {
        if definition.effects != Effects::None {
            assert_eq!(definition.folding, Folding::Never);
            assert_eq!(definition.purity, Purity::Impure);
        } else {
            assert_eq!(definition.folding, Folding::StaticArguments);
        }
    }
    for id in [IntrinsicId::Report, IntrinsicId::Assert, IntrinsicId::Now] {
        assert!(
            registry
                .definitions()
                .iter()
                .any(|definition| definition.id == id && definition.effects != Effects::None)
        );
    }
}

#[test]
fn native_declaration_fingerprint_signature_and_package_are_all_required() {
    let registry = IntrinsicRegistry::standard();
    let expected = registry
        .declaration(IntrinsicId::IntegerAdd)
        .unwrap()
        .unwrap();
    for change in 0..3 {
        let mut wrong = expected.clone();
        match change {
            0 => {
                wrong.declarations =
                    ContentFingerprint::of_bytes(b"not the trusted declaration set")
            }
            1 => wrong.signature.arguments[0] = NativeType::Boolean,
            2 => wrong.unit.primary = Identifier::Basic("user_package".into()),
            _ => unreachable!(),
        }
        assert!(registry.resolve(&wrong).unwrap().is_none());
    }
}

#[test]
fn native_standard_type_profiles_match_declared_order_and_osv_limits() {
    let registry = IntrinsicRegistry::standard();
    let find = |id| {
        &registry
            .types()
            .iter()
            .find(|definition| definition.id == id)
            .unwrap()
            .shape
    };
    assert_eq!(registry.types().len(), 18);
    assert_eq!(
        find(NativeType::Integer),
        &NativeTypeShape::Integer {
            minimum: i64::MIN,
            maximum: i64::MAX
        }
    );
    let NativeTypeShape::Enumeration { literals } = find(NativeType::Character) else {
        panic!("character must be an enumeration")
    };
    assert_eq!(literals.len(), 256);
    for (position, name) in [
        (0, "nul"),
        (31, "usp"),
        (127, "del"),
        (128, "c128"),
        (159, "c159"),
    ] {
        assert_eq!(
            literals[position],
            NativeLiteral::Identifier(Identifier::Basic(name.into()))
        );
    }
    assert_eq!(literals[65], NativeLiteral::Character(b'A'));
    assert_eq!(literals[255], NativeLiteral::Character(255));
    assert_eq!(
        find(NativeType::String),
        &NativeTypeShape::Array {
            index: NativeType::Positive,
            element: NativeType::Character
        }
    );
    let NativeTypeShape::Physical {
        minimum,
        maximum,
        units,
    } = find(NativeType::Time)
    else {
        panic!("time must be physical")
    };
    assert_eq!((*minimum, *maximum), (i64::MIN, i64::MAX));
    assert_eq!(units.len(), 8);
    assert_eq!(units[2].name, Identifier::Basic("ns".into()));
    assert_eq!(units[2].primary_units, 1_000_000);
    assert_eq!(units[7].primary_units, 3_600_000_000_000_000_000);
}

#[test]
fn native_basic_package_and_function_names_have_canonical_case_identity() {
    let registry = IntrinsicRegistry::standard();
    let mut declaration = registry.declaration(IntrinsicId::Now).unwrap().unwrap();
    declaration.unit.library = Identifier::Basic("STD".into());
    declaration.unit.primary = Identifier::Basic("STANDARD".into());
    declaration.signature.name = IntrinsicName::Function(Identifier::Basic("NOW".into()));
    assert_eq!(
        registry.resolve(&declaration).unwrap().unwrap().id,
        IntrinsicId::Now
    );
}
