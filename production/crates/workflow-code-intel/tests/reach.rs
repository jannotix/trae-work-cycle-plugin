use std::collections::{BTreeMap, BTreeSet};

use workflow_code_intel::graph::{
    EdgeInput, EdgeKind, FactConfidence, FactProvider, GraphEdge, GraphNode, GraphPartition,
    GraphStore, NodeInput, NodeKind, PartitionId, ReachConfidence,
};
use workflow_core::{ProjectId, WorkflowTimestamp};

/// One symbol per file, and an edge for every consumer named. Edges point consumer -> consumed,
/// which is the direction reach walks backwards.
fn graph(project_id: ProjectId, files: &[(&str, &[&str])]) -> Vec<GraphPartition> {
    let mut symbols: BTreeMap<String, GraphNode> = BTreeMap::new();
    for (path, _) in files {
        let scope = scope_of(path);
        let node = GraphNode::new(NodeInput {
            confidence: FactConfidence::Extracted,
            kind: NodeKind::Symbol,
            name: symbol_of(path),
            partition_id: PartitionId::new(project_id, &scope),
            provider: FactProvider::Parser("rust".to_owned()),
            qualified_name: (*path).to_owned(),
            range: None,
            source_path: (*path).to_owned(),
        })
        .unwrap();
        symbols.insert((*path).to_owned(), node);
    }
    let mut partitions: BTreeMap<String, GraphPartition> = BTreeMap::new();
    for (path, consumes) in files {
        let scope = scope_of(path);
        let partition = partitions
            .entry(scope.clone())
            .or_insert_with(|| GraphPartition {
                edges: BTreeMap::new(),
                external_nodes: BTreeSet::new(),
                id: PartitionId::new(project_id, &scope),
                nodes: BTreeMap::new(),
                project_id,
                scope: scope.clone(),
            });
        let consumer = symbols[*path].clone();
        partition.nodes.insert(consumer.id, consumer.clone());
        for consumed in *consumes {
            let edge = GraphEdge::new(EdgeInput {
                confidence: FactConfidence::Extracted,
                kind: EdgeKind::Calls,
                partition_id: partition.id,
                provider: FactProvider::Parser("rust".to_owned()),
                range: None,
                source: consumer.id,
                source_path: (*path).to_owned(),
                target: symbols[*consumed].id,
            })
            .unwrap();
            partition.edges.insert(edge.id, edge);
        }
    }
    for partition in partitions.values_mut() {
        for edge in partition.edges.values() {
            if !partition.nodes.contains_key(&edge.target) {
                partition.external_nodes.insert(edge.target);
            }
        }
    }
    partitions.into_values().collect()
}

fn scope_of(path: &str) -> String {
    path.rsplit_once('/')
        .map_or(".", |(head, _)| head)
        .to_owned()
}

fn symbol_of(path: &str) -> String {
    path.rsplit_once('/')
        .map_or(path, |(_, tail)| tail)
        .to_owned()
}

/// The graph lives in the control-plane database, so its schema comes from the store's migrations.
fn migrated(directory: &tempfile::TempDir) -> std::path::PathBuf {
    let path = directory.path().join("workflow.db");
    drop(workflow_store::Store::open(&path, std::num::NonZeroUsize::new(1).unwrap()).unwrap());
    path
}

fn stored(project_id: ProjectId, files: &[(&str, &[&str])]) -> (tempfile::TempDir, GraphStore) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = GraphStore::open(migrated(&directory)).unwrap();
    for partition in graph(project_id, files) {
        store
            .replace_partition(&partition, true, WorkflowTimestamp::now())
            .unwrap();
    }
    // The manifest is what says the project has been indexed at all.
    let entries: Vec<_> = files.iter().map(|(path, _)| manifest_entry(path)).collect();
    let scopes: BTreeSet<String> = files.iter().map(|(path, _)| scope_of(path)).collect();
    store
        .replace_manifest_scopes(project_id, &scopes, &entries)
        .unwrap();
    (directory, store)
}

fn manifest_entry(path: &str) -> workflow_code_intel::ManifestEntry {
    workflow_code_intel::ManifestEntry {
        content_hash: workflow_core::ContentDigest::of(path.as_bytes()),
        metadata: workflow_code_intel::FileMetadata {
            length: 1,
            modified_unix_nanos: None,
        },
        relative_path: path.to_owned(),
    }
}

#[test]
fn a_scope_reaches_what_consumes_it_across_partitions() {
    let project_id = ProjectId::new();
    let (_directory, store) = stored(
        project_id,
        &[
            ("src/config.rs", &[]),
            ("src/ui/page.tsx", &["src/config.rs"]),
            ("src/unrelated.rs", &[]),
        ],
    );

    let reach = store
        .reach_of_scopes(project_id, &BTreeSet::from(["src/config.rs".to_owned()]))
        .unwrap();

    assert_eq!(reach.confidence, ReachConfidence::Resolved);
    assert!(!reach.truncated);
    assert!(
        reach.paths.contains("src/ui/page.tsx"),
        "the interface file consuming the config must be reached: {:?}",
        reach.paths
    );
    assert!(
        !reach.paths.contains("src/config.rs"),
        "a scope does not reach itself"
    );
    assert!(
        !reach.paths.contains("src/unrelated.rs"),
        "reach is not the whole repository: {:?}",
        reach.paths
    );
}

#[test]
fn reach_follows_a_consumer_of_a_consumer_but_stops_there() {
    let project_id = ProjectId::new();
    let (_directory, store) = stored(
        project_id,
        &[
            ("src/a.rs", &[]),
            ("src/b.rs", &["src/a.rs"]),
            ("src/c.rs", &["src/b.rs"]),
            ("src/d.rs", &["src/c.rs"]),
        ],
    );

    let reach = store
        .reach_of_scopes(project_id, &BTreeSet::from(["src/a.rs".to_owned()]))
        .unwrap();

    assert!(reach.paths.contains("src/b.rs"), "the direct consumer");
    assert!(reach.paths.contains("src/c.rs"), "and its consumer");
    assert!(
        !reach.paths.contains("src/d.rs"),
        "past depth two the reached set is the repository: {:?}",
        reach.paths
    );
}

#[test]
fn an_unindexed_project_says_so_instead_of_reporting_nothing_affected() {
    let project_id = ProjectId::new();
    let directory = tempfile::tempdir().unwrap();
    let store = GraphStore::open(migrated(&directory)).unwrap();

    let reach = store
        .reach_of_scopes(project_id, &BTreeSet::from(["src/config.rs".to_owned()]))
        .unwrap();

    assert_eq!(reach.confidence, ReachConfidence::Unresolved);
    assert!(reach.paths.is_empty());
    assert!(
        reach
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("cycle_index")),
        "the reason names the command that resolves it: {:?}",
        reach.reason
    );
}

#[test]
fn a_scope_the_index_does_not_model_is_unresolved_rather_than_empty() {
    let project_id = ProjectId::new();
    let (_directory, store) = stored(project_id, &[("src/config.rs", &[])]);

    let reach = store
        .reach_of_scopes(project_id, &BTreeSet::from(["docs/manual.md".to_owned()]))
        .unwrap();

    assert_eq!(
        reach.confidence,
        ReachConfidence::Unresolved,
        "nothing modelled under the scope is an unknown, not a nothing"
    );
    assert!(reach.paths.is_empty());
}

#[test]
fn a_hub_is_reported_as_a_hub_instead_of_several_hundred_gates() {
    let project_id = ProjectId::new();
    // One shared symbol consumed by enough files to pass the ceiling.
    let consumers: Vec<String> = (0..260).map(|index| format!("src/c{index}.rs")).collect();
    let mut files: Vec<(&str, &[&str])> = vec![("src/logger.rs", &[])];
    let logger: &[&str] = &["src/logger.rs"];
    for consumer in &consumers {
        files.push((consumer.as_str(), logger));
    }
    let (_directory, store) = stored(project_id, &files);

    let reach = store
        .reach_of_scopes(project_id, &BTreeSet::from(["src/logger.rs".to_owned()]))
        .unwrap();

    assert_eq!(reach.confidence, ReachConfidence::Resolved);
    assert!(reach.truncated, "a hub truncates rather than expanding");
    assert!(
        reach.paths.is_empty(),
        "a truncated reach carries no paths to turn into gates"
    );
    assert_eq!(
        reach.hubs.first().map(|hub| hub.path.as_str()),
        Some("src/logger.rs")
    );
    assert!(
        reach.hubs.first().is_some_and(|hub| hub.consumers >= 260),
        "the hub names how many consume it: {:?}",
        reach.hubs.first()
    );
}
