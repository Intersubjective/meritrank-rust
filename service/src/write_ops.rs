use crate::aug_multi_graph::{AugMultiGraph, NodeInfo, NodeKind};
use crate::bloom_filter::{bloom_filter_add, bloom_filter_bits, bloom_filter_contains};
use crate::log::*;
use crate::nodes::node_name_from_id;
use meritrank_core::{NodeId, Weight, Cluster, constants::EPSILON, Graph};

// --- write_create_context ---
pub fn write_create_context(
    graph: &mut AugMultiGraph,
    context: &str,
) {
    log_command!("{:?}", context);
    graph.subgraph_from_context(context);
}

// --- write_put_edge ---
pub fn write_put_edge(
    graph: &mut AugMultiGraph,
    context: &str,
    src: &str,
    dst: &str,
    new_weight: f64,
    magnitude: i64,
) {
    log_command!(
        "{:?} {:?} {:?} {} {}",
        context, src, dst, new_weight, magnitude
    );

    if magnitude < 0 {
        log_verbose!(
            "Negative magnitude detected: context={}, src={}, dst={}, magnitude={}. Converting to 0.",
            context, src, dst, magnitude
        );
    }

    let mag_clamped = magnitude.max(0) as u32;
    let src_id = graph.find_or_add_node_by_name(src);
    let dst_id = graph.find_or_add_node_by_name(dst);
    let (
        new_weight_scaled,
        mut new_min_weight,
        new_max_weight,
        new_mag_scale,
        rescale_factor,
    ) = graph.vsids.scale_weight(context, src_id, new_weight, mag_clamped);

    let edge_deletion_threshold = new_max_weight * graph.vsids.deletion_ratio;
    let can_delete_at_least_one_edge = new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;

    if can_delete_at_least_one_edge || must_rescale {
        let (edges_to_modify, new_min_weight_from_scan) = graph
            .subgraph_from_context(context)
            .meritrank_data
            .graph
            .get_node_data(src_id)
            .unwrap()
            .get_outgoing_edges()
            .fold(
                (Vec::new(), new_min_weight),
                |(mut to_modify, min), (dest, weight_val)| {
                    let abs_weight = if must_rescale {
                        weight_val.abs() / rescale_factor
                    } else {
                        weight_val.abs()
                    };

                    if abs_weight <= edge_deletion_threshold {
                        to_modify.push((dest, 0.0));
                        (to_modify, min)
                    } else {
                        if must_rescale {
                            to_modify.push((dest, weight_val / rescale_factor));
                        }
                        (to_modify, min.min(abs_weight))
                    }
                },
            );
        new_min_weight = new_min_weight_from_scan;

        for (local_dst_id, local_weight_val) in edges_to_modify {
            log_verbose!(
                "Rescale or delete node: context={:?}, src={}, dst={}, new_weight={}",
                context,
                node_name_from_id(&graph.node_infos, src_id),
                node_name_from_id(&graph.node_infos, local_dst_id),
                local_weight_val
            );
            graph.set_edge(context, src_id, local_dst_id, local_weight_val);
        }
    }
    graph.set_edge(context, src_id, dst_id, new_weight_scaled);
    if must_rescale {
        log_verbose!(
            "Rescale performed: context={:?}, src={}, dst={}, normalized_new_weight={}",
            context, src, dst, new_weight_scaled
        );
    } else {
        log_verbose!(
            "Edge updated without rescale: context={:?}, src={}, dst={}, new_weight_scaled={}",
            context, src, dst, new_weight_scaled
        );
    }
    graph.vsids.min_max_weights.insert(
        (context.to_string(), src_id),
        (new_min_weight, new_max_weight, new_mag_scale),
    );
}

// --- write_delete_edge ---
pub fn write_delete_edge(
    graph: &mut AugMultiGraph,
    context: &str,
    src: &str,
    dst: &str,
    _index: i64,
) {
    log_command!("{:?} {:?} {:?}", context, src, dst);

    if !graph.node_exists(src) || !graph.node_exists(dst) {
        return;
    }

    let src_id = graph.find_or_add_node_by_name(src);
    let dst_id = graph.find_or_add_node_by_name(dst);

    graph.set_edge(context, src_id, dst_id, 0.0);
}

// --- write_delete_node ---
pub fn write_delete_node(
    graph: &mut AugMultiGraph,
    context: &str,
    node: &str,
    _index: i64,
) {
    log_command!("{:?} {:?}", context, node);

    if !graph.node_exists(node) {
        return;
    }

    let id = graph.find_or_add_node_by_name(node);

    let outgoing_edges: Vec<NodeId> = graph
        .subgraph_from_context(context)
        .meritrank_data
        .graph
        .get_node_data(id)
        .map(|data| {
            data.get_outgoing_edges()
                .into_iter()
                .map(|(n_id, _)| n_id)
                .collect()
        })
        .unwrap_or_else(Vec::new);

    for n_id_val in outgoing_edges {
        graph.set_edge(context, id, n_id_val, 0.0);
    }
}

// --- write_reset ---
pub fn write_reset(graph: &mut AugMultiGraph) {
    log_command!();
    graph.reset();
}

// --- write_new_edges_filter ---
pub fn write_new_edges_filter(
    graph: &mut AugMultiGraph,
    src: &str,
    filter_bytes: &[u8],
) {
    log_command!("{:?} {:?}", src, filter_bytes);

    let src_id = graph.find_or_add_node_by_name(src);

    let mut v_u64: Vec<u64> = vec![0; (filter_bytes.len() + 7) / 8];

    for i_byte in 0..filter_bytes.len() {
        v_u64[i_byte / 8] |= (filter_bytes[i_byte] as u64) << (8 * (i_byte % 8));
    }
    
    if (src_id as usize) < graph.node_infos.len() {
        graph.node_infos[src_id as usize].seen_nodes = v_u64;
    } else {
        log_error!("src_id {} is out of bounds for node_infos in write_new_edges_filter", src_id);
    }
}

// --- write_fetch_new_edges ---
pub fn write_fetch_new_edges(
    graph: &mut AugMultiGraph,
    src: &str,
    prefix: &str,
) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", src, prefix);

    let num_hashes = graph.settings.filter_num_hashes;
    let max_size = graph.settings.filter_max_size / 8; 

    let src_id = graph.find_or_add_node_by_name(src);

    if (src_id as usize) >= graph.node_infos.len() {
         log_error!("src_id {} is out of bounds for node_infos in write_fetch_new_edges. This should not happen if find_or_add_node_by_name ensures capacity.", src_id);
         return Vec::new();
    }

    if graph.node_infos[src_id as usize].seen_nodes.is_empty() {
        let min_size_u64 = (graph.settings.filter_min_size + 7) / 8;
        let initial_size = std::cmp::min(min_size_u64.max(1), max_size.max(1)); // Ensure initial_size is at least 1 and not exceeding max_size
        graph.node_infos[src_id as usize].seen_nodes.resize(initial_size, 0);
        log_verbose!(
            "Create the bloom filter with {} bytes for {:?}",
            8 * graph.node_infos[src_id as usize].seen_nodes.len(),
            src
        );
    }

    let mut v_results = Vec::<(String, Weight, Weight, Cluster, Cluster)>::new();

    for dst_id_loop in 0..graph.node_count {
        if (dst_id_loop as usize) >= graph.node_infos.len() || !graph.node_infos[dst_id_loop as usize].name.starts_with(prefix) {
            continue;
        }

        let (score_value_of_dst, score_cluster_of_dst) =
            graph.fetch_score("", src_id, dst_id_loop); // Context is empty string
        let (score_value_of_src, score_cluster_of_src) =
            graph.fetch_score_cached("", src_id, dst_id_loop); // Context is empty string

        if score_value_of_dst < EPSILON {
            continue;
        }

        let current_filter_len = graph.node_infos[src_id as usize].seen_nodes.len();
        if current_filter_len == 0 {
             log_error!("Bloom filter for {} is empty during contains check.", src);
             continue; // Avoid panic with bloom_filter_bits if len is 0
        }
        let bits = bloom_filter_bits(current_filter_len, num_hashes, dst_id_loop);

        if !bloom_filter_contains(&graph.node_infos[src_id as usize].seen_nodes, &bits) {
            v_results.push((
                graph.node_infos[dst_id_loop as usize].name.clone(),
                score_value_of_dst,
                score_value_of_src,
                score_cluster_of_dst,
                score_cluster_of_src,
            ));
        }
    }

    let mut seen_nodes_rebuild = vec![];
    let current_len = graph.node_infos[src_id as usize].seen_nodes.len();
    // Ensure initial_rebuild_size is at least 1 if max_size is > 0, or 0 if max_size is 0.
    let initial_rebuild_size = if max_size == 0 { 0 } else { std::cmp::min(current_len.max(1), max_size) };
    seen_nodes_rebuild.resize(initial_rebuild_size, 0);
    
    if max_size > 0 { // Proceed with rebuild only if max_size allows for a filter
        loop {
            let mut saturated = false;
            for x_u64 in seen_nodes_rebuild.iter_mut() {
                *x_u64 = 0;
            }

            for dst_id_rebuild in 0..graph.node_count {
                if (dst_id_rebuild as usize) >= graph.node_infos.len() { continue; }

                let current_rebuild_filter_len = seen_nodes_rebuild.len();
                if current_rebuild_filter_len == 0 { // Should not happen if initial_rebuild_size is at least 1
                    log_error!("Rebuild bloom filter for {} is empty during bits generation.", src);
                    continue;
                }
                let bits = bloom_filter_bits(current_rebuild_filter_len, num_hashes, dst_id_rebuild);
                
                let should_add_to_filter = {
                    if graph.node_infos[dst_id_rebuild as usize].name.starts_with(prefix) {
                        let num_walks = graph.settings.num_walks;
                        let k_factor = graph.settings.zero_opinion_factor;
                        let score = graph
                            .subgraph_from_context("") // Context is empty string
                            .fetch_raw_score(src_id, dst_id_rebuild, num_walks, k_factor);
                        score >= EPSILON
                    } else {
                        let len_original_filter = graph.node_infos[src_id as usize].seen_nodes.len();
                        if len_original_filter > 0 { // Check original filter only if it's not empty
                            let bits_original = bloom_filter_bits(len_original_filter, num_hashes, dst_id_rebuild);
                            bloom_filter_contains(&graph.node_infos[src_id as usize].seen_nodes, &bits_original)
                        } else {
                            false
                        }
                    }
                };

                if should_add_to_filter {
                    let collision = bloom_filter_contains(&seen_nodes_rebuild, &bits);
                    if collision && current_rebuild_filter_len < max_size {
                        let n_new_size = std::cmp::min(current_rebuild_filter_len * 2, max_size);
                        if n_new_size > current_rebuild_filter_len { // Only resize if it actually grows
                            seen_nodes_rebuild.resize(n_new_size, 0);
                            log_verbose!(
                                "Resize the bloom filter to {} bytes for {:?}",
                                8 * n_new_size, src
                            );
                            saturated = true;
                            break; 
                        }
                    }
                    // Add to filter if not saturated (or if saturated but resize didn't happen/wasn't needed)
                    if !saturated { // This check might be redundant if break happens
                       bloom_filter_add(&mut seen_nodes_rebuild, &bits);
                    }
                }
            }

            if !saturated {
                if seen_nodes_rebuild.len() >= max_size {
                    log_warning!("Max bloom filer size is reached for {:?}", src);
                }
                graph.node_infos[src_id as usize].seen_nodes = seen_nodes_rebuild;
                break;
            }
        }
    } else { // max_size is 0, so clear the filter
        graph.node_infos[src_id as usize].seen_nodes.clear();
        log_warning!("Bloom filter for {} was cleared because max_size is 0.", src);
    }

    // Rebuild the bloom filter
    rebuild_bloom_filter_for_source(graph, src, src_id, prefix);

    // Return fetched edges
    v_results
}

// Helper function to rebuild bloom filter for a source node
pub(crate) fn rebuild_bloom_filter_for_source(
    graph: &mut AugMultiGraph,
    src_for_logging: &str, // Renamed from src to avoid conflict
    src_id: NodeId,
    prefix: &str,
) {
    let num_hashes = graph.settings.filter_num_hashes;
    let max_size = graph.settings.filter_max_size / 8; // filter_max_size is in bits

    let mut seen_nodes_rebuild = vec![];
    // Ensure src_id is within bounds before accessing node_infos
    if (src_id as usize) >= graph.node_infos.len() {
        log_error!("src_id {} is out of bounds for node_infos in rebuild_bloom_filter_for_source", src_id);
        return;
    }
    let current_len = graph.node_infos[src_id as usize].seen_nodes.len();
    let initial_rebuild_size = if max_size == 0 { 0 } else { std::cmp::min(current_len.max(1), max_size) };
    seen_nodes_rebuild.resize(initial_rebuild_size, 0);

    if max_size > 0 { // Proceed with rebuild only if max_size allows for a filter
        loop {
            let mut saturated = false;
            for x_u64 in seen_nodes_rebuild.iter_mut() {
                *x_u64 = 0;
            }

            for dst_id_rebuild in 0..graph.node_count {
                if (dst_id_rebuild as usize) >= graph.node_infos.len() { continue; }

                let current_rebuild_filter_len = seen_nodes_rebuild.len();
                if current_rebuild_filter_len == 0 {
                    log_error!("Rebuild bloom filter for {} is empty during bits generation.", src_for_logging);
                    continue;
                }
                let bits = bloom_filter_bits(current_rebuild_filter_len, num_hashes, dst_id_rebuild);
                
                let should_add_to_filter = {
                    if graph.node_infos[dst_id_rebuild as usize].name.starts_with(prefix) {
                        let num_walks = graph.settings.num_walks;
                        let k_factor = graph.settings.zero_opinion_factor;
                        let score = graph
                            .subgraph_from_context("") // Context is empty string
                            .fetch_raw_score(src_id, dst_id_rebuild, num_walks, k_factor);
                        score >= EPSILON
                    } else {
                        let len_original_filter = graph.node_infos[src_id as usize].seen_nodes.len();
                        if len_original_filter > 0 {
                            let bits_original = bloom_filter_bits(len_original_filter, num_hashes, dst_id_rebuild);
                            bloom_filter_contains(&graph.node_infos[src_id as usize].seen_nodes, &bits_original)
                        } else {
                            false
                        }
                    }
                };

                if should_add_to_filter {
                    let collision = bloom_filter_contains(&seen_nodes_rebuild, &bits);
                    if collision && current_rebuild_filter_len < max_size {
                        let n_new_size = std::cmp::min(current_rebuild_filter_len * 2, max_size);
                        if n_new_size > current_rebuild_filter_len {
                            seen_nodes_rebuild.resize(n_new_size, 0);
                            log_verbose!(
                                "Resize the bloom filter to {} bytes for {:?}",
                                8 * n_new_size, src_for_logging // Use src_for_logging
                            );
                            saturated = true;
                            break; 
                        }
                    }
                    if !saturated {
                       bloom_filter_add(&mut seen_nodes_rebuild, &bits);
                    }
                }
            }

            if !saturated {
                if seen_nodes_rebuild.len() >= max_size {
                    log_warning!("Max bloom filer size is reached for {:?}", src_for_logging); // Use src_for_logging
                }
                graph.node_infos[src_id as usize].seen_nodes = seen_nodes_rebuild;
                break;
            }
        }
    } else { // max_size is 0, so clear the filter
        graph.node_infos[src_id as usize].seen_nodes.clear();
        log_warning!("Bloom filter for {} was cleared because max_size is 0.", src_for_logging); // Use src_for_logging
    }
}

// --- write_set_zero_opinion ---
pub fn write_set_zero_opinion(
    graph: &mut AugMultiGraph,
    context: &str,
    node: &str,
    score: Weight,
) {
    log_command!("{:?} {} {}", context, node, score);
    let id = graph.find_or_add_node_by_name(node);
    let zero_opinion_ref = &mut graph.subgraph_from_context(context).zero_opinion;

    if (id as usize) >= zero_opinion_ref.len() {
        zero_opinion_ref.resize((id + 1) as usize, 0.0);
    }
    zero_opinion_ref[id as usize] = score;
}
