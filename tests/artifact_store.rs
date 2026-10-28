#[path = "support/artifact.rs"]
mod support;

use std::{fs, io::Write, path::Path};

use osv::artifacts::{store::*, *};

fn architecture(name: &str) -> UnitIdentity {
    UnitIdentity {
        library: Identifier::Basic("work".into()),
        primary: Identifier::Basic("synthetic".into()),
        kind: UnitKind::Architecture {
            name: Identifier::Basic(name.into()),
        },
    }
}

#[test]
fn catalog_distinguishes_architectures_by_full_unit_identity() {
    let catalog = LibraryCatalog {
        version: CATALOG_VERSION,
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
        library: Identifier::Basic("work".into()),
        entries: ["first", "second"]
            .map(|name| CatalogEntry {
                identity: architecture(name),
                fingerprint: ContentFingerprint::of_bytes(name.as_bytes()),
                language: LanguageRevision::Vhdl2008,
            })
            .to_vec(),
        bindings: ["first", "second"]
            .map(|name| ActiveBinding {
                identity: architecture(name),
                fingerprint: ContentFingerprint::of_bytes(name.as_bytes()),
            })
            .to_vec(),
    };
    assert_eq!(
        catalog.lookup(&architecture("SECOND")).unwrap().fingerprint,
        ContentFingerprint::of_bytes(b"second")
    );
    assert!(catalog.lookup(&architecture("absent")).is_none());
}

fn open(root: &Path) -> LibraryStore {
    LibraryStore::open(
        root,
        Identifier::Basic("work".into()),
        StoreOptions::default(),
    )
    .unwrap()
}

fn named(name: &str, kind: UnitKind) -> ArtifactEnvelope {
    let mut artifact = support::sample();
    artifact.identity.primary = Identifier::Basic(name.into());
    artifact.identity.kind = kind;
    artifact.units[0].identity = artifact.identity.clone();
    artifact
}

fn dependency(artifact: &ArtifactEnvelope, role: DependencyRole) -> DependencyRecord {
    DependencyRecord {
        identity: artifact.identity.clone(),
        stage: ArtifactStage::Analysis,
        role,
        fingerprint: artifact.fingerprint().unwrap(),
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
    }
}

fn secondary(primary: &ArtifactEnvelope, kind: UnitKind, marker: &str) -> ArtifactEnvelope {
    let mut artifact = primary.clone();
    artifact.identity.kind = kind;
    artifact.units.push(UnitRecord {
        id: UnitId(8),
        identity: artifact.identity.clone(),
        parent: Some(UnitId(7)),
        span: None,
    });
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(2),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Unit(UnitId(8)),
    });
    artifact.symbols.clear();
    artifact.types.clear();
    artifact.payload = ArtifactPayload::Analysis(AnalysisArtifact {
        unit: UnitId(8),
        declarations: Vec::new(),
        objects: Vec::new(),
        pending_processes: Vec::new(),
    });
    artifact.dependencies = vec![dependency(primary, DependencyRole::Declaration)];
    let mut file = artifact.sources[0].file.clone();
    file.bytes = Some(format!("-- synthetic {marker}\n").into_bytes());
    artifact.sources[0] = SourceRecord::embedded(file).unwrap();
    artifact
}

#[test]
fn artifact_store_fresh_reused_and_reopened_entries_are_exact() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let artifact = support::sample();
    assert!(matches!(
        store.resolve_current(&artifact.identity),
        Err(StoreError::Missing(_))
    ));
    let fingerprint = store.publish(&artifact, ArtifactFormat::Binary).unwrap();
    let original = fs::read(store.artifact_path(fingerprint)).unwrap();
    assert_eq!(
        store.publish(&artifact, ArtifactFormat::Ron).unwrap(),
        fingerprint
    );
    assert_eq!(store.catalog().entries.len(), 1);
    assert_eq!(store.catalog().bindings.len(), 1);
    assert_eq!(
        fs::read(store.artifact_path(fingerprint)).unwrap(),
        original
    );
    let reopened = open(directory.path());
    let graph = reopened.resolve_current(&artifact.identity).unwrap();
    assert_eq!(graph.root, fingerprint);
    assert_eq!(graph.artifacts, vec![artifact.canonicalized().unwrap()]);
}

#[test]
fn artifact_store_declaration_changes_invalidate_current_but_preserve_exact_graphs() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let mut package = named("definitions", UnitKind::Package);
    store.publish(&package, ArtifactFormat::Ron).unwrap();
    let mut consumer = named("consumer", UnitKind::Entity);
    consumer.dependencies = vec![dependency(&package, DependencyRole::Declaration)];
    let consumer_fingerprint = store.publish(&consumer, ArtifactFormat::Binary).unwrap();
    assert_eq!(
        store
            .resolve_current(&consumer.identity)
            .unwrap()
            .artifacts
            .len(),
        2
    );
    package.symbols[0].name = Identifier::Basic("changed_declaration".into());
    store.publish(&package, ArtifactFormat::Binary).unwrap();
    assert!(matches!(
        store.resolve_current(&consumer.identity),
        Err(StoreError::Stale(_))
    ));
    assert_eq!(
        store
            .resolve_exact(&consumer.identity, consumer_fingerprint)
            .unwrap()
            .artifacts
            .len(),
        2
    );
    assert_eq!(
        store
            .catalog()
            .entries
            .iter()
            .filter(|entry| entry.identity == package.identity)
            .count(),
        2
    );
}

#[test]
fn artifact_store_body_changes_do_not_invalidate_specification_only_consumers() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let package = named("definitions", UnitKind::Package);
    store.publish(&package, ArtifactFormat::Binary).unwrap();
    let first = secondary(&package, UnitKind::PackageBody, "body one");
    let first_fingerprint = store.publish(&first, ArtifactFormat::Ron).unwrap();
    let mut consumer = named("consumer", UnitKind::Entity);
    consumer.dependencies = vec![dependency(&package, DependencyRole::Declaration)];
    store.publish(&consumer, ArtifactFormat::Binary).unwrap();
    let second = secondary(&package, UnitKind::PackageBody, "body two");
    let second_fingerprint = store.publish(&second, ArtifactFormat::Binary).unwrap();
    assert_ne!(first_fingerprint, second_fingerprint);
    assert_eq!(
        store
            .resolve_current(&consumer.identity)
            .unwrap()
            .artifacts
            .len(),
        2
    );
    assert_eq!(
        store
            .resolve_exact(&first.identity, first_fingerprint)
            .unwrap()
            .root,
        first_fingerprint
    );
    consumer
        .dependencies
        .push(dependency(&first, DependencyRole::Body));
    let identity = consumer.identity.clone();
    store.publish(&consumer, ArtifactFormat::Binary).unwrap();
    assert!(matches!(
        store.resolve_current(&identity),
        Err(StoreError::Stale(_))
    ));
}

#[test]
fn artifact_store_architectures_are_independent_secondary_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let entity = support::sample();
    store.publish(&entity, ArtifactFormat::Binary).unwrap();
    for name in ["first", "second"] {
        let artifact = secondary(
            &entity,
            UnitKind::Architecture {
                name: Identifier::Basic(name.into()),
            },
            name,
        );
        let fingerprint = store.publish(&artifact, ArtifactFormat::Ron).unwrap();
        let graph = store.resolve_current(&artifact.identity).unwrap();
        assert_eq!(graph.root, fingerprint);
        assert_eq!(graph.artifacts.len(), 2);
    }
    assert_eq!(store.catalog().entries.len(), 3);
}

#[test]
fn artifact_store_cross_library_dependencies_resolve_without_sources() {
    let directory = tempfile::tempdir().unwrap();
    let mut other = named("definitions", UnitKind::Package);
    other.identity.library = Identifier::Basic("other".into());
    other.scopes[0].kind = ScopeKind::Library(other.identity.library.clone());
    other.units[0].identity = other.identity.clone();
    let mut other_store = LibraryStore::open(
        directory.path(),
        other.identity.library.clone(),
        StoreOptions::default(),
    )
    .unwrap();
    other_store.publish(&other, ArtifactFormat::Ron).unwrap();
    let mut consumer = support::sample();
    consumer
        .dependencies
        .push(dependency(&other, DependencyRole::Declaration));
    let mut store = open(directory.path());
    store.publish(&consumer, ArtifactFormat::Binary).unwrap();
    assert_eq!(
        open(directory.path())
            .resolve_current(&consumer.identity)
            .unwrap()
            .artifacts
            .len(),
        2
    );
}

#[test]
fn artifact_store_missing_references_and_corrupt_objects_are_failures() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let missing = named("absent", UnitKind::Package);
    let mut artifact = support::sample();
    artifact
        .dependencies
        .push(dependency(&missing, DependencyRole::Declaration));
    assert!(matches!(
        store.publish(&artifact, ArtifactFormat::Binary),
        Err(StoreError::Missing(_))
    ));
    artifact.dependencies.clear();
    let fingerprint = store.publish(&artifact, ArtifactFormat::Binary).unwrap();
    fs::write(store.artifact_path(fingerprint), b"corrupt").unwrap();
    assert!(store.resolve_current(&artifact.identity).is_err());
    fs::remove_file(store.artifact_path(fingerprint)).unwrap();
    assert!(store.resolve_current(&artifact.identity).is_err());
}

#[test]
fn artifact_store_stale_writers_cannot_lose_catalog_updates() {
    let directory = tempfile::tempdir().unwrap();
    let mut first = open(directory.path());
    let mut second = open(directory.path());
    let artifact = support::sample();
    first.publish(&artifact, ArtifactFormat::Binary).unwrap();
    let another = named("another", UnitKind::Entity);
    assert!(matches!(
        second.publish(&another, ArtifactFormat::Ron),
        Err(StoreError::ConcurrentUpdate)
    ));
    assert_eq!(open(directory.path()).catalog().entries.len(), 1);
}

#[test]
fn artifact_store_ignores_interrupted_temporary_and_orphan_object_writes() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let artifact = support::sample();
    store.publish(&artifact, ArtifactFormat::Binary).unwrap();
    let before = fs::read(store.catalog_path().unwrap()).unwrap();
    {
        let catalog_path = store.catalog_path().unwrap();
        let mut interrupted =
            tempfile::NamedTempFile::new_in(catalog_path.parent().unwrap()).unwrap();
        interrupted.write_all(b"incomplete catalog").unwrap();
        assert_eq!(open(directory.path()).catalog().entries.len(), 1);
    }
    let orphan = named("orphan", UnitKind::Entity);
    let fingerprint = orphan.fingerprint().unwrap();
    publish_new(
        &store.artifact_path(fingerprint),
        &encode(&orphan, ArtifactFormat::Binary).unwrap(),
    )
    .unwrap();
    let reopened = open(directory.path());
    assert!(matches!(
        reopened.resolve_current(&orphan.identity),
        Err(StoreError::Missing(_))
    ));
    assert_eq!(fs::read(store.catalog_path().unwrap()).unwrap(), before);
}

#[test]
fn artifact_store_native_schema_and_graph_limits_are_enforced() {
    let directory = tempfile::tempdir().unwrap();
    let requirement = NativeRequirement {
        library: Identifier::Basic("synthetic_lib".into()),
        package: Identifier::Basic("native".into()),
        declarations: ContentFingerprint::of_bytes(b"first declaration set"),
        abi_version: NATIVE_ABI_VERSION,
    };
    let mut artifact = support::sample();
    artifact.native_requirements.push(requirement.clone());
    let options = StoreOptions {
        native_requirements: vec![requirement],
        ..StoreOptions::default()
    };
    let mut store = LibraryStore::open(
        directory.path(),
        artifact.identity.library.clone(),
        options.clone(),
    )
    .unwrap();
    store.publish(&artifact, ArtifactFormat::Binary).unwrap();
    assert!(matches!(
        open(directory.path()).resolve_current(&artifact.identity),
        Err(StoreError::NativeCompatibility)
    ));
    let mut changed_native = options.clone();
    changed_native.native_requirements[0].declarations =
        ContentFingerprint::of_bytes(b"changed declaration set");
    let changed = LibraryStore::open(
        directory.path(),
        artifact.identity.library.clone(),
        changed_native,
    )
    .unwrap();
    assert!(matches!(
        changed.resolve_current(&artifact.identity),
        Err(StoreError::NativeCompatibility)
    ));
    let limited = LibraryStore::open(
        directory.path(),
        artifact.identity.library.clone(),
        StoreOptions {
            max_graph_bytes: 1,
            ..options.clone()
        },
    )
    .unwrap();
    assert!(matches!(
        limited.resolve_current(&artifact.identity),
        Err(StoreError::ResourceLimit)
    ));
    let path = store.catalog_path().unwrap();
    let bytes = fs::read(&path).unwrap();
    let mut catalog: LibraryCatalog = postcard::from_bytes(&bytes[8..]).unwrap();
    catalog.schema_version += 1;
    let mut incompatible = bytes[..8].to_vec();
    incompatible.extend(postcard::to_allocvec(&catalog).unwrap());
    fs::write(path, incompatible).unwrap();
    assert!(matches!(
        LibraryStore::open(directory.path(), artifact.identity.library.clone(), options),
        Err(StoreError::InvalidCatalog(_))
    ));
}

#[test]
fn artifact_store_combined_version_conflicts_fail_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let first = named("definitions", UnitKind::Package);
    store.publish(&first, ArtifactFormat::Binary).unwrap();
    let mut second = first.clone();
    second.symbols[0].name = Identifier::Basic("second".into());
    store.publish(&second, ArtifactFormat::Binary).unwrap();
    let mut intermediary = named("intermediary", UnitKind::Package);
    intermediary
        .dependencies
        .push(dependency(&second, DependencyRole::Declaration));
    store
        .publish(&intermediary, ArtifactFormat::Binary)
        .unwrap();
    let mut consumer = named("consumer", UnitKind::Entity);
    consumer.dependencies = vec![
        dependency(&first, DependencyRole::Declaration),
        dependency(&intermediary, DependencyRole::Declaration),
    ];
    assert!(matches!(
        store.publish(&consumer, ArtifactFormat::Binary),
        Err(StoreError::VersionConflict(_))
    ));
    assert!(store.catalog().lookup(&consumer.identity).is_none());
}

#[test]
fn artifact_store_graph_node_limits_and_diagnostics_are_explicit() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = open(directory.path());
    let package = named("definitions", UnitKind::Package);
    store.publish(&package, ArtifactFormat::Binary).unwrap();
    let mut consumer = support::sample();
    consumer
        .dependencies
        .push(dependency(&package, DependencyRole::Declaration));
    store.publish(&consumer, ArtifactFormat::Binary).unwrap();
    let limited = LibraryStore::open(
        directory.path(),
        consumer.identity.library.clone(),
        StoreOptions {
            max_graph_artifacts: 1,
            ..StoreOptions::default()
        },
    )
    .unwrap();
    let error = limited.resolve_current(&consumer.identity).err().unwrap();
    assert!(matches!(error, StoreError::ResourceLimit));
    assert_eq!(
        error.diagnostic().code,
        osv::diagnostics::DiagnosticCode::ArtifactLimit
    );
    assert_eq!(error.diagnostic().exit_status(), 1);
}
