//! What a set of authorized write scopes reaches, and how sure the graph is of it.
//!
//! This feeds the evidence policy and never the routing. Adding paths to the set the gate rules
//! are matched against can only insert a gate, never remove one, so the answer is promote-only by
//! construction: being wrong costs a proof that was not needed, never one that was.
//!
//! "Resolved" is claimed narrowly, because the failure worth avoiding is not "I cannot tell" -- it
//! is "I can tell", and being wrong. A scope the index does not hold makes the whole answer
//! unresolved with the reason named, rather than quietly contributing nothing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rusqlite::params;

use super::{GraphStore, GraphStoreError, NodeId};
use workflow_core::ProjectId;

/// A direct consumer and its consumer. Past that the reached set is the repository.
const DEPTH: usize = 2;

/// How wide a reached set may get before it stops being information. Past this the answer is not
/// "each of these needs a proof" but "this change touches a hub", which is a finding for the
/// reviewers rather than several hundred gates. Whichever is larger, because a small repository
/// has no hubs to speak of and a large one should not be judged by an absolute count.
const MAX_REACHED: usize = 200;
const MAX_REACHED_PROPORTION: usize = 10;

/// Seeds are bounded too: a scope covering the whole repository would otherwise walk all of it.
const MAX_SEEDS: usize = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReachConfidence {
    /// Every scope was one the index holds, and the reached set is what the graph says.
    Resolved,
    /// The graph could not answer for at least one scope. Says so rather than reporting nothing.
    Unresolved,
}

/// A touched symbol many other files consume: what "this is a hub" means concretely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReachHub {
    pub consumers: u64,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reach {
    pub confidence: ReachConfidence,
    /// Named only when the reached set was too wide to expand into gates.
    pub hubs: Vec<ReachHub>,
    /// Files the scopes reach but do not contain. Empty when unresolved or truncated.
    pub paths: BTreeSet<String>,
    /// Why the reach is unresolved, and what would resolve it. None when resolved.
    pub reason: Option<String>,
    pub truncated: bool,
}

impl Reach {
    fn unresolved(reason: String) -> Self {
        Self {
            confidence: ReachConfidence::Unresolved,
            hubs: Vec::new(),
            paths: BTreeSet::new(),
            reason: Some(reason),
            truncated: false,
        }
    }
}

impl GraphStore {
    /// What the files under these write scopes reach through the graph.
    ///
    /// `scopes` are repository-relative, as the architecture plan declares them. A scope naming a
    /// directory covers everything under it; a scope naming a file covers that file.
    pub fn reach_of_scopes(
        &self,
        project_id: ProjectId,
        scopes: &BTreeSet<String>,
    ) -> Result<Reach, GraphStoreError> {
        if scopes.is_empty() {
            return Ok(Reach {
                confidence: ReachConfidence::Resolved,
                hubs: Vec::new(),
                paths: BTreeSet::new(),
                reason: None,
                truncated: false,
            });
        }
        let indexed_files = self.indexed_file_count(project_id)?;
        if indexed_files == 0 {
            return Ok(Reach::unresolved(
                "this project has never been indexed, so what the change reaches is unknown. Run \
                 cycle_index to resolve it."
                    .to_owned(),
            ));
        }
        let seeds = self.nodes_under_scopes(project_id, scopes)?;
        if seeds.len() > MAX_SEEDS {
            return Ok(Reach::unresolved(format!(
                "the authorized write scopes cover {} indexed symbols, more than the {MAX_SEEDS} \
                 this can expand; narrow the scopes or treat the change as repository-wide",
                seeds.len()
            )));
        }
        if seeds.is_empty() {
            // Nothing the graph models lives under these scopes. That is a legitimate answer for
            // documentation or configuration, and a gap for source the index should have held --
            // the two are indistinguishable from here, so neither is claimed.
            return Ok(Reach::unresolved(
                "the index holds no symbols under the authorized write scopes, so what they reach \
                 is unknown. Run cycle_index to resolve it."
                    .to_owned(),
            ));
        }
        let seed_ids: BTreeSet<String> = seeds.keys().map(ToString::to_string).collect();
        let touched: BTreeSet<String> = seeds.values().map(|(path, _)| path.clone()).collect();
        let ceiling = MAX_REACHED
            .max(usize::try_from(indexed_files).unwrap_or(usize::MAX) / MAX_REACHED_PROPORTION);
        let reached = self.consumer_paths(project_id, &seed_ids, ceiling)?;
        let paths: BTreeSet<String> = reached
            .paths
            .into_iter()
            .filter(|path| !touched.contains(path))
            .collect();
        if reached.truncated || paths.len() > ceiling {
            return Ok(Reach {
                confidence: ReachConfidence::Resolved,
                hubs: self.hubs_of(project_id, &seeds)?,
                paths: BTreeSet::new(),
                reason: None,
                truncated: true,
            });
        }
        Ok(Reach {
            confidence: ReachConfidence::Resolved,
            hubs: Vec::new(),
            paths,
            reason: None,
            truncated: false,
        })
    }

    fn indexed_file_count(&self, project_id: ProjectId) -> Result<u64, GraphStoreError> {
        let count: i64 = self.connection.query_row(
            "SELECT count(*) FROM code_manifest WHERE project_id = ?1",
            [project_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// Nodes defined in files under any of the scopes, at each partition's head generation.
    fn nodes_under_scopes(
        &self,
        project_id: ProjectId,
        scopes: &BTreeSet<String>,
    ) -> Result<BTreeMap<NodeId, (String, String)>, GraphStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT nodes.node_json
             FROM code_nodes nodes
             JOIN code_partitions partitions
               ON partitions.id = nodes.partition_id
              AND partitions.head_generation = nodes.generation
             WHERE partitions.project_id = ?1",
        )?;
        let mut found = BTreeMap::new();
        let rows = statement.query_map([project_id.to_string()], |row| row.get::<_, String>(0))?;
        for row in rows {
            let json = row?;
            let node: super::GraphNode = serde_json::from_str(&json)?;
            if scopes.iter().any(|scope| covers(scope, &node.source_path)) {
                found.insert(node.id, (node.source_path, node.name));
            }
            if found.len() > MAX_SEEDS {
                break;
            }
        }
        Ok(found)
    }

    /// Files consuming these nodes, breadth-first to `DEPTH`, stopping at `ceiling`.
    fn consumer_paths(
        &self,
        project_id: ProjectId,
        seeds: &BTreeSet<String>,
        ceiling: usize,
    ) -> Result<ConsumerPaths, GraphStoreError> {
        // The consuming file lives inside edge_json rather than in a column, so each edge on the
        // frontier is deserialized. The ceiling below is what keeps that bounded.
        let mut statement = self.connection.prepare(
            "SELECT edges.source_id, edges.edge_json
             FROM code_edges edges
             JOIN code_partitions partitions
               ON partitions.id = edges.partition_id
              AND partitions.head_generation = edges.generation
             WHERE partitions.project_id = ?1 AND edges.target_id = ?2",
        )?;
        let mut paths = BTreeSet::new();
        let mut visited: BTreeSet<String> = seeds.clone();
        let mut queue: VecDeque<(String, usize)> =
            seeds.iter().map(|node| (node.clone(), 0_usize)).collect();
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= DEPTH {
                continue;
            }
            let rows = statement.query_map(params![project_id.to_string(), node], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (source_id, edge_json) = row?;
                let edge: super::GraphEdge = serde_json::from_str(&edge_json)?;
                paths.insert(edge.source_path);
                if paths.len() > ceiling {
                    return Ok(ConsumerPaths {
                        paths,
                        truncated: true,
                    });
                }
                if visited.insert(source_id.clone()) {
                    queue.push_back((source_id, depth + 1));
                }
            }
        }
        Ok(ConsumerPaths {
            paths,
            truncated: false,
        })
    }

    /// The touched symbols with the most consumers, which is what a hub finding names.
    fn hubs_of(
        &self,
        project_id: ProjectId,
        seeds: &BTreeMap<NodeId, (String, String)>,
    ) -> Result<Vec<ReachHub>, GraphStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT count(DISTINCT edges.source_id)
             FROM code_edges edges
             JOIN code_partitions partitions
               ON partitions.id = edges.partition_id
              AND partitions.head_generation = edges.generation
             WHERE partitions.project_id = ?1 AND edges.target_id = ?2",
        )?;
        let mut hubs = Vec::new();
        for (node, (path, name)) in seeds {
            let consumers: i64 = statement
                .query_row(params![project_id.to_string(), node.to_string()], |row| {
                    row.get(0)
                })?;
            let consumers = u64::try_from(consumers).unwrap_or(0);
            if consumers > 0 {
                hubs.push(ReachHub {
                    consumers,
                    name: name.clone(),
                    path: path.clone(),
                });
            }
        }
        hubs.sort_by(|left, right| {
            right
                .consumers
                .cmp(&left.consumers)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.name.cmp(&right.name))
        });
        hubs.truncate(10);
        Ok(hubs)
    }
}

struct ConsumerPaths {
    paths: BTreeSet<String>,
    truncated: bool,
}

/// A scope covers a path when it is the path, or a directory prefix of it. Both sides are
/// normalized to forward slashes because plans are written by hand on either platform.
fn covers(scope: &str, path: &str) -> bool {
    let scope = scope.replace('\\', "/");
    let scope = scope.trim_end_matches('/');
    let path = path.replace('\\', "/");
    if scope.is_empty() || scope == "." {
        return true;
    }
    path == scope || path.starts_with(&format!("{scope}/"))
}

#[cfg(test)]
mod tests {
    use super::covers;

    #[test]
    fn a_scope_covers_itself_and_what_is_under_it_but_never_a_sibling_prefix() {
        assert!(covers("src/auth", "src/auth/session.rs"));
        assert!(covers("src/auth/", "src/auth/session.rs"));
        assert!(covers("src/auth/session.rs", "src/auth/session.rs"));
        assert!(covers("src\\auth", "src/auth/session.rs"));
        // "src/auth" must not swallow "src/authorization": the shared prefix is not a directory.
        assert!(!covers("src/auth", "src/authorization/policy.rs"));
        assert!(!covers("src/auth", "src/ui/page.tsx"));
    }
}
