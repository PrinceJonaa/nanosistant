//! Graph theory primitives — pure deterministic functions.

// ═══════════════════════════════════════
// Centrality & Clustering
// ═══════════════════════════════════════

/// Degree centrality of a node in an undirected adjacency matrix.
/// = degree(node) / (n - 1)
#[must_use]
pub fn degree_centrality(node: usize, adj_matrix: &[Vec<bool>]) -> f64 {
    let n = adj_matrix.len();
    if n <= 1 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let degree = adj_matrix[node].iter().filter(|&&x| x).count() as f64;
    degree / (n - 1) as f64
}

/// Local clustering coefficient: fraction of a node's neighbours that are
/// connected to each other.
#[must_use]
pub fn clustering_coefficient(node: usize, adj_matrix: &[Vec<bool>]) -> f64 {
    let n = adj_matrix.len();
    if node >= n { return 0.0; }
    let neighbours: Vec<usize> = (0..n)
        .filter(|&j| j != node && adj_matrix[node][j])
        .collect();
    let k = neighbours.len();
    if k < 2 { return 0.0; }
    let mut links = 0usize;
    for i in 0..k {
        for j in (i + 1)..k {
            if adj_matrix[neighbours[i]][neighbours[j]] { links += 1; }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let result = 2.0 * links as f64 / (k * (k - 1)) as f64;
    result
}

// ═══════════════════════════════════════
// Graph Structure
// ═══════════════════════════════════════

/// Graph density: actual edges / max possible edges.
#[must_use]
pub fn density(node_count: usize, edge_count: usize, directed: bool) -> f64 {
    if node_count < 2 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let max_edges = if directed {
        (node_count * (node_count - 1)) as f64
    } else {
        (node_count * (node_count - 1)) as f64 / 2.0
    };
    edge_count as f64 / max_edges
}

/// Rough diameter estimate via BFS depth ≈ ln(n) / ln(avg_degree).
#[must_use]
pub fn diameter_hint(node_count: usize, avg_degree: f64) -> f64 {
    if node_count <= 1 || avg_degree <= 1.0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let d = (node_count as f64).ln() / avg_degree.ln();
    d
}

/// A connected acyclic graph has exactly node_count - 1 edges.
#[must_use]
pub fn is_tree(node_count: usize, edge_count: usize) -> bool {
    node_count > 0 && edge_count == node_count.saturating_sub(1)
}

/// A complete graph has n*(n-1)/2 undirected edges (or n*(n-1) directed).
#[must_use]
pub fn is_complete_graph(node_count: usize, edge_count: usize, directed: bool) -> bool {
    if node_count == 0 { return edge_count == 0; }
    let max = if directed {
        node_count * (node_count - 1)
    } else {
        node_count * (node_count - 1) / 2
    };
    edge_count == max
}

// ═══════════════════════════════════════
// Small World & Social
// ═══════════════════════════════════════

/// Characteristic path length threshold for small-world networks: ln(n).
#[must_use]
pub fn small_world_threshold(node_count: usize) -> f64 {
    if node_count == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let t = (node_count as f64).ln();
    t
}

/// Dunbar's layer sizes: 1→5, 2→15, 3→50, 4→150, 5→500.
#[must_use]
pub fn dunbar_layer(layer: u8) -> u32 {
    match layer {
        1 => 5,
        2 => 15,
        3 => 50,
        4 => 150,
        5 => 500,
        _ => 0,
    }
}

/// Estimated degrees of separation = log(n) / log(avg_k).
#[must_use]
pub fn six_degrees_path_estimate(node_count: usize) -> f64 {
    // Uses average degree ≈ 6 (Milgram-era estimate)
    if node_count <= 1 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let est = (node_count as f64).ln() / 6.0_f64.ln();
    est
}

/// Rough betweenness estimate: scales as O(n*(n-1)/2).
#[must_use]
pub fn betweenness_estimate(node_count: usize) -> f64 {
    if node_count < 2 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let est = (node_count * (node_count - 1)) as f64 / 2.0;
    est
}

/// Erdős–Rényi edge probability for connectivity threshold: ln(n)/n.
#[must_use]
pub fn erdos_renyi_edge_prob_for_connectivity(node_count: usize) -> f64 {
    if node_count <= 1 { return 1.0; }
    #[allow(clippy::cast_precision_loss)]
    let p = (node_count as f64).ln() / node_count as f64;
    p
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> Vec<Vec<bool>> {
        // 3-node complete graph (triangle)
        vec![
            vec![false, true,  true ],
            vec![true,  false, true ],
            vec![true,  true,  false],
        ]
    }

    fn path3() -> Vec<Vec<bool>> {
        // 0 — 1 — 2
        vec![
            vec![false, true,  false],
            vec![true,  false, true ],
            vec![false, true,  false],
        ]
    }

    #[test] fn test_degree_centrality_triangle() {
        // Each node in a triangle has degree 2 out of max 2
        let g = triangle();
        let dc = degree_centrality(0, &g);
        assert!((dc - 1.0).abs() < 1e-10, "expected 1.0, got {dc}");
    }

    #[test] fn test_degree_centrality_path() {
        let g = path3();
        let dc_center = degree_centrality(1, &g); // degree=2, n-1=2
        assert!((dc_center - 1.0).abs() < 1e-10);
        let dc_end = degree_centrality(0, &g); // degree=1
        assert!((dc_end - 0.5).abs() < 1e-10);
    }

    #[test] fn test_clustering_triangle() {
        // In a triangle, all 3 neighbours of each node are mutually connected
        let g = triangle();
        let cc = clustering_coefficient(0, &g);
        assert!((cc - 1.0).abs() < 1e-10, "triangle cc should be 1.0, got {cc}");
    }

    #[test] fn test_clustering_path() {
        // Middle node in path: neighbours (0 and 2) are NOT connected
        let g = path3();
        let cc = clustering_coefficient(1, &g);
        assert!((cc - 0.0).abs() < 1e-10, "path middle cc should be 0.0, got {cc}");
    }

    #[test] fn test_density_undirected() {
        // Triangle: 3 nodes, 3 edges, max = 3
        let d = density(3, 3, false);
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test] fn test_density_directed() {
        // 3 nodes, 3 directed edges, max = 6
        let d = density(3, 3, true);
        assert!((d - 0.5).abs() < 1e-10);
    }

    #[test] fn test_diameter_hint() {
        // ln(1000) / ln(10) ≈ 3
        let h = diameter_hint(1000, 10.0);
        assert!((h - 3.0).abs() < 0.01);
    }

    #[test] fn test_is_tree() {
        assert!( is_tree(5, 4));
        assert!(!is_tree(5, 5));
        assert!( is_tree(1, 0));
    }

    #[test] fn test_is_complete_graph() {
        assert!( is_complete_graph(4, 6, false));  // K4
        assert!(!is_complete_graph(4, 5, false));
        assert!( is_complete_graph(4, 12, true));  // directed K4
        assert!( is_complete_graph(0, 0, false));
    }

    #[test] fn test_small_world_threshold() {
        let t = small_world_threshold(1000);
        assert!((t - 1000_f64.ln()).abs() < 1e-10);
        assert_eq!(small_world_threshold(0), 0.0);
    }

    #[test] fn test_dunbar_layer() {
        assert_eq!(dunbar_layer(1), 5);
        assert_eq!(dunbar_layer(2), 15);
        assert_eq!(dunbar_layer(3), 50);
        assert_eq!(dunbar_layer(4), 150);
        assert_eq!(dunbar_layer(5), 500);
        assert_eq!(dunbar_layer(6), 0);
    }

    #[test] fn test_six_degrees() {
        // For 1e9 nodes, estimate ≈ ln(1e9)/ln(6) ≈ 12.8
        let est = six_degrees_path_estimate(1_000_000_000);
        assert!(est > 10.0 && est < 20.0, "got {est}");
    }

    #[test] fn test_betweenness_estimate() {
        assert_eq!(betweenness_estimate(0), 0.0);
        assert_eq!(betweenness_estimate(1), 0.0);
        assert!((betweenness_estimate(4) - 6.0).abs() < 1e-10);
    }

    #[test] fn test_erdos_renyi_threshold() {
        // ln(100)/100 ≈ 0.046
        let p = erdos_renyi_edge_prob_for_connectivity(100);
        assert!(p > 0.04 && p < 0.06, "got {p}");
    }
}
