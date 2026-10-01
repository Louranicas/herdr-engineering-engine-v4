//! Custody controls over a tiny, explicitly UNMEASURED representation fixture.
//! No producer, checker, design reviewer, or protected collector ran for this data.
//! Preparation is separately predeclared, never reconstructed from the receipt.
//! Pending-publication preflight remains source reviewed: no timing hook is used.

use habitat_engine::app::{
    evidence::Evidence,
    receipt_import::{Error, receipt as import},
};
use habitat_engine::check::consistency::Editable;
use habitat_engine::check::patch::CandidateBounds;
use habitat_engine::check::{
    collector::Sink,
    consistency::{CasePlan, Prepared, Summary},
    graph::{Graph, Objects},
};
use habitat_engine::contracts::receipt::RelPath;
use habitat_engine::contracts::{
    UuidV4,
    receipt::{
        ExpectationV1, Id, IdentityV1, InvocationV1, List, Name, ReceiptV1, Ref, Sha, SubjectsV1,
        TypedRef, VerdictV1State, decode,
    },
};
use habitat_engine::store::{ArtifactStaging, Store};
use serde::Deserialize;
use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const GENERATION: &str = "74000000-0000-4000-8000-000000000001";
const EPOCH: &str = "74000000-0000-4000-8000-000000000002";
const RESTORE_ID: &str = "74000000-0000-4000-8000-000000000003";
const OBJECTS: usize = 28;
const OBJECT_BYTES: u64 = 16_310;
static NEXT_AREA: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureObject {
    label: String,
    reference: Ref,
    bytes: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    root: TypedRef<ReceiptV1>,
    objects: Vec<FixtureObject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlannedCase {
    case_id: Name,
    primary_module_id: Name,
    criterion_ids: List<Name>,
    fixture_sha256: Sha,
    oracle_id: Name,
    expected: TypedRef<ExpectationV1>,
    mandatory: bool,
    selected: bool,
    excluded: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    schema_sha256: Sha,
    identity: IdentityV1,
    subjects: SubjectsV1,
    invocation: InvocationV1,
    cases: Vec<PlannedCase>,
}

fn preparation() -> Prepared {
    let value: Preparation =
        serde_json::from_str(include_str!("fixtures/receipt-import/preparation.json")).unwrap();
    Prepared {
        schema_sha256: value.schema_sha256,
        identity: value.identity,
        subjects: value.subjects,
        invocation: value.invocation,
        cases: value
            .cases
            .into_iter()
            .map(|case| CasePlan {
                case_id: case.case_id,
                primary_module_id: case.primary_module_id,
                criterion_ids: case.criterion_ids,
                fixture_sha256: case.fixture_sha256,
                oracle_id: case.oracle_id,
                expected: case.expected,
                mandatory: case.mandatory,
                selected: case.selected,
                excluded: case.excluded,
                reviewed_design: None,
            })
            .collect(),
        // The imported receipts are the WL-U64 lane's, whose one editable file this is.
        editable: Editable {
            path: RelPath::new("src/lib.rs").unwrap(),
            bounds: CandidateBounds {
                bytes: 65_536,
                changed_lines: 200,
            },
        },
    }
}

impl Fixture {
    fn object(&self, label: &str) -> &FixtureObject {
        self.objects.iter().find(|row| row.label == label).unwrap()
    }
}

struct Area {
    path: PathBuf,
    device: u64,
    inode: u64,
}
impl Area {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "hee3-t06-receipt-import-{}-{}",
            std::process::id(),
            NEXT_AREA.fetch_add(1, Ordering::Relaxed)
        ));
        private_directory(&path);
        let metadata = fs::symlink_metadata(&path).unwrap();
        Self {
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}
impl Drop for Area {
    fn drop(&mut self) {
        let metadata = fs::symlink_metadata(&self.path).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        assert_eq!((metadata.dev(), metadata.ino()), (self.device, self.inode));
        fs::remove_dir_all(&self.path).unwrap();
    }
}
fn private_directory(path: &Path) {
    DirBuilder::new().mode(0o700).create(path).unwrap();
    fs::set_permissions(path, Permissions::from_mode(0o700)).unwrap();
}

struct Context<'a> {
    fixture: &'a Fixture,
    prepared: Prepared,
    staging: &'a ArtifactStaging,
    source: Evidence<'a>,
    owner: &'a Store,
    destination: Evidence<'a>,
    directory: &'a Path,
    deadline: Instant,
}

fn run(control: impl FnOnce(&mut Context<'_>)) {
    let prepared = preparation();
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/receipt-import/nonpass.json")).unwrap();
    assert_eq!(fixture.objects.len(), OBJECTS);
    assert!(fixture.objects.iter().all(|row| {
        row.reference == *fixture.root.as_ref()
            || fixture.root.as_ref().artifact_id.as_str() < row.reference.artifact_id.as_str()
    }));
    let root: ReceiptV1 = decode(fixture.object("root").bytes.as_bytes()).unwrap();
    assert_eq!(root.verdict.state, VerdictV1State::Unmeasured);
    assert_eq!(root.cases.primary_credit, 0);
    let area = Area::new();
    let stage_path = area.path.join("staging");
    let destination_path = area.path.join("destination");
    private_directory(&stage_path);
    private_directory(&destination_path);
    let deadline = Instant::now() + Duration::from_secs(60);
    let staging = ArtifactStaging::open(&stage_path, true, deadline).unwrap();
    let owner = Store::open(
        &destination_path,
        UuidV4::parse(GENERATION).unwrap(),
        UuidV4::parse(EPOCH).unwrap(),
        true,
        deadline,
    )
    .unwrap();
    let mut source = Evidence::staged(&staging, deadline);
    for row in &fixture.objects {
        assert_eq!(
            source
                .publish(&row.reference, row.bytes.as_bytes())
                .unwrap(),
            row.reference
        );
    }
    let graph = Graph::resolve(&source, fixture.root.as_ref()).unwrap();
    assert_eq!(graph.object_count(), OBJECTS);
    assert_eq!(graph.total_bytes(), OBJECT_BYTES);
    let mut context = Context {
        fixture: &fixture,
        prepared,
        staging: &staging,
        source,
        owner: &owner,
        destination: Evidence::new(&owner, deadline),
        directory: &destination_path,
        deadline,
    };
    control(&mut context);
}

fn import_ok(context: &mut Context<'_>) {
    assert_eq!(
        import(
            &mut context.destination,
            &context.source,
            &context.prepared,
            &context.fixture.root
        )
        .unwrap(),
        Summary {
            cases: 1,
            primary_credit: 0,
            mandatory_cases: 0
        }
    );
    assert_eq!(context.destination.registered().len(), OBJECTS);
    assert!(context.destination.pending_publications().is_empty());
    for row in &context.fixture.objects {
        let mut bytes = Vec::new();
        context
            .destination
            .open(&row.reference)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, row.bytes.as_bytes());
        assert_eq!(
            context.destination.registered()[row.reference.artifact_id.as_str()].0,
            row.reference
        );
    }
}

fn object_path(directory: &Path, reference: &Ref) -> PathBuf {
    let digest = &reference.sha256.as_str()[7..];
    directory
        .join("generations")
        .join(GENERATION)
        .join("objects/sha256")
        .join(&digest[..2])
        .join(digest)
}
fn root_absent(context: &Context<'_>) {
    assert!(
        !context
            .destination
            .registered()
            .contains_key(context.fixture.root.as_ref().artifact_id.as_str())
    );
    assert!(!object_path(context.directory, context.fixture.root.as_ref()).exists());
}
fn owner_restore(context: &Context<'_>, row: &FixtureObject) {
    context
        .owner
        .publish(
            row.bytes.as_bytes(),
            UuidV4::parse(RESTORE_ID).unwrap(),
            context.deadline,
        )
        .unwrap();
}

#[test]
fn exact_closure_preserves_all_ref_fields_and_excludes_unreachable_input() {
    run(|context| {
        let extra = context
            .source
            .payload(b"unreachable sentinel", "text/plain")
            .unwrap();
        import_ok(context);
        assert!(
            !context
                .destination
                .registered()
                .contains_key(extra.as_ref().artifact_id.as_str())
        );
        assert!(!object_path(context.directory, extra.as_ref()).exists());
    });
}

#[test]
fn exact_replay_keeps_registry_and_object_bytes_unchanged() {
    run(|context| {
        import_ok(context);
        let before = context.destination.registered().clone();
        import_ok(context);
        assert_eq!(context.destination.registered(), &before);
    });
}

#[test]
fn changed_preparation_refuses_before_any_publication() {
    run(|context| {
        let mut changed = context.prepared.clone();
        changed.identity.run_id = Id::new("75000000-0000-4000-8000-000000000001").unwrap();
        assert!(matches!(
            import(
                &mut context.destination,
                &context.source,
                &changed,
                &context.fixture.root
            ),
            Err(Error::Consistency(_))
        ));
        assert!(context.destination.registered().is_empty());
        root_absent(context);
        import_ok(context);
    });
}

#[test]
fn conflicting_full_reference_refuses_before_new_writes() {
    run(|context| {
        let row = context.fixture.object("late_payload");
        let mut conflicting = row.reference.clone();
        conflicting.media_type = Name::new("application/x-fictional-conflict").unwrap();
        context
            .destination
            .publish(&conflicting, row.bytes.as_bytes())
            .unwrap();
        let before = context.destination.registered().clone();
        assert_eq!(
            import(
                &mut context.destination,
                &context.source,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Conflict)
        );
        assert_eq!(context.destination.registered(), &before);
        root_absent(context);
        // The owner rebuilds its ephemeral registry using the intended exact Ref.
        let object = before[conflicting.artifact_id.as_str()].1.clone();
        context.destination = Evidence::new(context.owner, context.deadline);
        context
            .destination
            .register(row.reference.clone(), object)
            .unwrap();
        import_ok(context);
    });
}

#[test]
fn missing_transitive_source_refuses_before_destination_writes() {
    run(|context| {
        let omitted = &context.fixture.object("seed_bytes").reference.artifact_id;
        let mut incomplete = Evidence::staged(context.staging, context.deadline);
        for (reference, object) in context.source.registered().values() {
            if &reference.artifact_id != omitted {
                incomplete
                    .register(reference.clone(), object.clone())
                    .unwrap();
            }
        }
        assert!(matches!(
            import(
                &mut context.destination,
                &incomplete,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Graph(_))
        ));
        assert!(context.destination.registered().is_empty());
        root_absent(context);
        import_ok(context);
    });
}

#[test]
fn expired_destination_retains_failure_without_publishing() {
    run(|context| {
        let expired = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        context.destination = Evidence::new(context.owner, expired);
        assert!(matches!(
            import(
                &mut context.destination,
                &context.source,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Publication(_))
        ));
        assert!(context.destination.registered().is_empty());
        assert!(context.destination.last_publication_error().is_some());
        root_absent(context);
        context.destination = Evidence::new(context.owner, context.deadline);
        import_ok(context);
    });
}

#[test]
fn missing_registered_root_requires_explicit_owner_reconciliation() {
    run(|context| {
        import_ok(context);
        let before = context.destination.registered().clone();
        let row = context.fixture.object("root");
        let path = object_path(context.directory, &row.reference);
        fs::remove_file(&path).unwrap();
        assert!(matches!(
            import(
                &mut context.destination,
                &context.source,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Graph(_))
        ));
        assert_eq!(context.destination.registered(), &before);
        assert!(!path.exists());
        owner_restore(context, row);
        import_ok(context);
    });
}

#[test]
fn missing_existing_child_is_checked_before_any_new_write() {
    run(|context| {
        let row = context.fixture.object("seed_bytes");
        context
            .destination
            .publish(&row.reference, row.bytes.as_bytes())
            .unwrap();
        let path = object_path(context.directory, &row.reference);
        fs::remove_file(&path).unwrap();
        let before = context.destination.registered().clone();
        assert!(matches!(
            import(
                &mut context.destination,
                &context.source,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Graph(_))
        ));
        assert_eq!(context.destination.registered(), &before);
        root_absent(context);
        for other in &context.fixture.objects {
            assert!(!object_path(context.directory, &other.reference).exists());
        }
        owner_restore(context, row);
        import_ok(context);
    });
}

#[test]
fn late_child_corruption_retains_partial_children_without_root_then_retry_succeeds() {
    run(|context| {
        let row = context.fixture.object("late_payload");
        assert!(context.fixture.objects.iter().all(|other| {
            other.reference == row.reference
                || other.reference.artifact_id.as_str() < row.reference.artifact_id.as_str()
        }));
        assert_eq!(
            context
                .fixture
                .objects
                .iter()
                .filter(|other| other.reference.sha256 == row.reference.sha256)
                .count(),
            1
        );
        let path = object_path(context.directory, &row.reference);
        if !path.parent().unwrap().exists() {
            private_directory(path.parent().unwrap());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(b"deliberately corrupt preexisting CAS bytes")
            .unwrap();
        file.sync_all().unwrap();
        drop(file);
        assert!(matches!(
            import(
                &mut context.destination,
                &context.source,
                &context.prepared,
                &context.fixture.root
            ),
            Err(Error::Publication(_))
        ));
        root_absent(context);
        assert_eq!(context.destination.registered().len(), OBJECTS - 2);
        assert!(context.destination.last_publication_error().is_some());
        assert!(context.destination.pending_publications().is_empty());
        // Only the test owner removes its deliberately introduced corrupt file.
        fs::remove_file(path).unwrap();
        import_ok(context);
    });
}
