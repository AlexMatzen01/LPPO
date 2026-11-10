//! LPPO package dependency resolver.
//!
//! Builds a directed dependency graph from `~/.LPPO/packages.json`, resolves
//! compatible versions, detects cycles and conflicts, and produces an
//! installation plan ordered so dependencies are built before dependents.

use crate::error::{LppoError, LppoResult};
use crate::pkg::types::{InstallPlan, PackageLibrary, PackageSource, VersionReq};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A node in the dependency graph.
#[derive(Debug, Clone)]
struct GraphNode {
    name: String,
    /// Required version constraint from the package that depends on this node.
    req: Option<VersionReq>,
    /// Dependencies declared by this package (name -> requirement).
    deps: BTreeMap<String, VersionReq>,
}

/// The dependency resolver.
pub struct DependencyResolver<'a> {
    library: &'a PackageLibrary,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(library: &'a PackageLibrary) -> Self {
        Self { library }
    }

    /// Resolve an installation plan starting from the given root package names.
    ///
    /// Returns packages in dependency order (dependencies before dependents).
    /// Detects circular dependencies and version conflicts.
    pub fn resolve(
        &self,
        roots: &[String],
        pinned: &BTreeMap<String, String>,
    ) -> LppoResult<InstallPlan> {
        let mut nodes: BTreeMap<String, GraphNode> = BTreeMap::new();
        let mut work: VecDeque<String> = VecDeque::new();

        for root in roots {
            if !work.contains(root) {
                work.push_back(root.clone());
            }
        }

        // Build the graph by expanding dependencies.
        while let Some(name) = work.pop_front() {
            if nodes.contains_key(&name) {
                continue;
            }
            let entry = self
                .library
                .packages
                .get(&name)
                .ok_or_else(|| LppoError::package_not_found(name.clone()))?;
            let mut deps = BTreeMap::new();
            for (dep_name, req_str) in &entry.dependencies {
                let req = VersionReq::parse(req_str).map_err(|e| {
                    LppoError::invalid_dependency(name.clone(), dep_name.clone(), e)
                })?;
                deps.insert(dep_name.clone(), req);
                if !nodes.contains_key(dep_name) && !work.contains(dep_name) {
                    work.push_back(dep_name.clone());
                }
            }
            nodes.insert(
                name.clone(),
                GraphNode {
                    name: name.clone(),
                    req: pinned.get(&name).and_then(|v| VersionReq::parse(v).ok()),
                    deps,
                },
            );
        }

        // Detect cycles.
        self.detect_cycles(&nodes)?;

        // Determine selected versions: use pinned if given, else library version.
        let mut versions = BTreeMap::new();
        let mut sources = BTreeMap::new();
        for (name, node) in &nodes {
            let version = if let Some(pinned_ver) = pinned.get(name) {
                pinned_ver.clone()
            } else {
                let entry = self
                    .library
                    .packages
                    .get(name)
                    .ok_or_else(|| LppoError::package_not_found(name.clone()))?;
                entry.version.clone()
            };
            // Validate the selected version satisfies any constraint imposed on it.
            if let Some(req) = &node.req {
                let parsed = crate::pkg::types::SemVer::parse(&version).map_err(|e| {
                    LppoError::version_conflict(name.clone(), version.clone(), e.to_string())
                })?;
                if !req.satisfies(&parsed) {
                    return Err(LppoError::version_conflict(
                        name.clone(),
                        version.clone(),
                        format!("does not satisfy constraint {req}"),
                    ));
                }
            }
            versions.insert(name.clone(), version);
            sources.insert(name.clone(), self.library_source(name)?);
        }

        // Topological sort: dependencies before dependents.
        let order = topological_sort(&nodes)?;

        Ok(InstallPlan {
            order,
            versions,
            sources,
        })
    }

    fn library_source(&self, name: &str) -> LppoResult<PackageSource> {
        self.library
            .packages
            .get(name)
            .map(|e| e.source.clone())
            .ok_or_else(|| LppoError::package_not_found(name.to_string()))
    }

    /// Detect cycles in the dependency graph using DFS. Reports the full chain.
    fn detect_cycles(&self, nodes: &BTreeMap<String, GraphNode>) -> LppoResult<()> {
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut stack: BTreeSet<String> = BTreeSet::new();
        let mut path: Vec<String> = Vec::new();

        fn dfs(
            name: &str,
            nodes: &BTreeMap<String, GraphNode>,
            visited: &mut BTreeSet<String>,
            stack: &mut BTreeSet<String>,
            path: &mut Vec<String>,
        ) -> LppoResult<Option<Vec<String>>> {
            if stack.contains(name) {
                // Found a cycle: extract the cycle portion of the path.
                let cycle_start = path.iter().position(|n| n == name).unwrap();
                let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                cycle.push(name.to_string());
                return Ok(Some(cycle));
            }
            if visited.contains(name) {
                return Ok(None);
            }
            visited.insert(name.to_string());
            stack.insert(name.to_string());
            path.push(name.to_string());

            let node = nodes.get(name).unwrap();
            for dep in node.deps.keys() {
                if let Some(cycle) = dfs(dep, nodes, visited, stack, path)? {
                    return Ok(Some(cycle));
                }
            }

            path.pop();
            stack.remove(name);
            Ok(None)
        }

        for name in nodes.keys() {
            if !visited.contains(name) {
                if let Some(cycle) = dfs(name, nodes, &mut visited, &mut stack, &mut path)? {
                    return Err(LppoError::circular_dependency(cycle));
                }
            }
        }
        Ok(())
    }
}

/// Topological sort the dependency graph so dependencies come before dependents.
fn topological_sort(nodes: &BTreeMap<String, GraphNode>) -> LppoResult<Vec<String>> {
    // Compute in-degrees (number of dependents... actually we want edges
    // dependency -> dependent, so in-degree of a node = number of packages
    // that depend on it... no.
    //
    // We want: if A depends on B, then B comes before A.
    // Edge: B -> A (B must precede A).
    // in-degree[A] = number of dependencies of A.
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, node) in nodes {
        in_degree.insert(name.clone(), node.deps.len());
        for dep in node.deps.keys() {
            dependents.entry(dep.clone()).or_default().push(name.clone());
        }
        in_degree.entry(name.clone()).or_default();
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (name, deg) in &in_degree {
        if *deg == 0 {
            queue.push_back(name.clone());
        }
    }

    let mut result = Vec::new();
    while let Some(name) = queue.pop_front() {
        result.push(name.clone());
        for dependent in dependents.get(&name).cloned().unwrap_or_default() {
            let deg = in_degree.get_mut(&dependent).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(dependent.clone());
            }
        }
    }

    if result.len() != nodes.len() {
        // Should not happen because cycles are detected earlier, but be safe.
        let remaining: BTreeSet<String> = nodes.keys().cloned().collect();
        let done: BTreeSet<String> = result.iter().cloned().collect();
        let mut missing: Vec<String> = remaining.difference(&done).cloned().collect();
        missing.sort();
        return Err(LppoError::circular_dependency(missing));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::{PackageEntry, PackageSource};
    use std::collections::BTreeMap;

    fn make_library() -> PackageLibrary {
        let mut lib = PackageLibrary::default();
        // LPPXPlatform has no deps.
        lib.packages.insert("LPPXPlatform".to_string(), PackageEntry {
            name: "LPPXPlatform".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/LPPX".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: Some("common".to_string()),
            }),
            lppo_version: None,
            dependencies: BTreeMap::new(),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        // LPPXCommon depends on LPPXPlatform.
        lib.packages.insert("LPPXCommon".to_string(), PackageEntry {
            name: "LPPXCommon".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/LPPX".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: Some("common".to_string()),
            }),
            lppo_version: None,
            dependencies: BTreeMap::from([("LPPXPlatform".to_string(), "^1.0.0".to_string())]),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        // LPPXMath depends on LPPXCommon.
        lib.packages.insert("LPPXMath".to_string(), PackageEntry {
            name: "LPPXMath".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/LPPX".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: Some("LPPXMath".to_string()),
            }),
            lppo_version: None,
            dependencies: BTreeMap::from([("LPPXCommon".to_string(), "^1.0.0".to_string())]),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        // LPPX2D depends on LPPXCommon and LPPXMath.
        lib.packages.insert("LPPX2D".to_string(), PackageEntry {
            name: "LPPX2D".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/LPPX".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: Some("LPPX2D".to_string()),
            }),
            lppo_version: None,
            dependencies: BTreeMap::from([
                ("LPPXCommon".to_string(), "^1.0.0".to_string()),
                ("LPPXMath".to_string(), "^1.0.0".to_string()),
            ]),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        lib
    }

    #[test]
    fn resolves_transitive_dependencies_in_order() {
        let lib = make_library();
        let resolver = DependencyResolver::new(&lib);
        let plan = resolver.resolve(&["LPPX2D".to_string()], &BTreeMap::new()).unwrap();
        // LPPXPlatform must come before LPPXCommon before LPPXMath/LPPX2D.
        let platform_idx = plan.order.iter().position(|n| n == "LPPXPlatform").unwrap();
        let common_idx = plan.order.iter().position(|n| n == "LPPXCommon").unwrap();
        let math_idx = plan.order.iter().position(|n| n == "LPPXMath").unwrap();
        let lppx2d_idx = plan.order.iter().position(|n| n == "LPPX2D").unwrap();
        assert!(platform_idx < common_idx);
        assert!(common_idx < math_idx);
        assert!(common_idx < lppx2d_idx);
        assert!(math_idx < lppx2d_idx);
        assert_eq!(plan.order.len(), 4);
    }

    #[test]
    fn detects_missing_package() {
        let lib = PackageLibrary::default();
        let resolver = DependencyResolver::new(&lib);
        let err = resolver
            .resolve(&["NoSuchPackage".to_string()], &BTreeMap::new())
            .unwrap_err();
        assert!(err.to_string().contains("NoSuchPackage"));
    }

    #[test]
    fn detects_circular_dependency() {
        let mut lib = PackageLibrary::default();
        lib.packages.insert("PkgA".to_string(), PackageEntry {
            name: "PkgA".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/x".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: None,
            }),
            lppo_version: None,
            dependencies: BTreeMap::from([("PkgB".to_string(), "^1.0.0".to_string())]),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        lib.packages.insert("PkgB".to_string(), PackageEntry {
            name: "PkgB".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            license: None,
            repository: None,
            homepage: None,
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/x".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: None,
            }),
            lppo_version: None,
            dependencies: BTreeMap::from([("PkgA".to_string(), "^1.0.0".to_string())]),
            build: Default::default(),
            integrity: None,
            metadata: BTreeMap::new(),
        });
        let resolver = DependencyResolver::new(&lib);
        let err = resolver
            .resolve(&["PkgA".to_string()], &BTreeMap::new())
            .unwrap_err();
        assert!(
            err.to_string().contains("circular"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn independent_roots_do_not_force_each_other() {
        let lib = make_library();
        let resolver = DependencyResolver::new(&lib);
        let plan = resolver
            .resolve(
                &["LPPXPlatform".to_string(), "LPPXMath".to_string()],
                &BTreeMap::new(),
            )
            .unwrap();
        assert!(plan.order.contains(&"LPPXPlatform".to_string()));
        assert!(plan.order.contains(&"LPPXMath".to_string()));
        // LPPXCommon should appear once (deduplicated).
        let common_count = plan.order.iter().filter(|n| n.as_str() == "LPPXCommon").count();
        assert_eq!(common_count, 1);
    }
}
