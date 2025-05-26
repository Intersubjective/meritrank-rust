use crate::aug_multi_graph::{AugMultiGraph, NodeKind, NodeInfo};
use meritrank_core::{Cluster, NodeId, Weight, constants::EPSILON};
use crate::log::*;
use crate::nodes::{node_kind_from_id, node_name_from_id, kind_from_prefix};
use crate::protocol::{NEIGHBORS_INBOUND, neighbor_dir_from};
use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use crate::astar_utils; 
use crate::subgraph::Subgraph;

// --- read_node_score (already moved and verified) ---
pub fn read_node_score(
    graph: &mut AugMultiGraph,
    context: &str,
    ego: &str,
    dst: &str,
) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?} {:?}", context, ego, dst);

    if !graph.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !graph.node_exists(ego) {
        log_error!("Node does not exist: {:?}", ego);
        return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !graph.node_exists(dst) {
        log_error!("Node does not exist: {:?}", dst);
        return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    let ego_id = graph.find_or_add_node_by_name(ego);
    let dst_id = graph.find_or_add_node_by_name(dst);

    let (score_of_dst_from_ego, score_cluster_of_dst) =
        graph.fetch_score(context, ego_id, dst_id);

    let (score_of_ego_from_dst, score_cluster_of_ego) =
        match graph.get_object_owner(context, dst_id) {
            Some(dst_owner_id) => {
                graph.fetch_score_cached(context, dst_owner_id, ego_id)
            },
            None => (0.0, 0), 
        };

    [(
        ego.to_string(),
        node_name_from_id(&graph.node_infos, dst_id),
        score_of_dst_from_ego,
        score_of_ego_from_dst,
        score_cluster_of_dst,
        score_cluster_of_ego,
    )]
    .to_vec()
}

// --- apply_filters_and_pagination (helper) ---
pub(crate) fn apply_filters_and_pagination(
    graph: &mut AugMultiGraph,
    scores: Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego: &str, 
    ego_id: NodeId,
    kind: NodeKind,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
    prioritize_ego_owned_nodes: bool,
) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    let mut im: Vec<(NodeId, Weight, Cluster)> = scores
        .into_iter()
        .map(|(n, w, cluster)| {
            (n, node_kind_from_id(&graph.node_infos, n), w, cluster)
        })
        .filter(|(_, target_kind, _, _)| {
            kind == NodeKind::Unknown || kind == *target_kind
        })
        .filter(|(_, _, score, _)| {
            score_gt < *score || (score_gte && score_gt <= *score)
        })
        .filter(|(_, _, score, _)| {
            *score < score_lt || (score_lte && score_lt >= *score)
        })
        .collect::<Vec<(NodeId, NodeKind, Weight, Cluster)>>()
        .into_iter()
        .filter(|(target_id, target_kind, _, _)| {
            if !hide_personal
                || (*target_kind != NodeKind::Comment
                    && *target_kind != NodeKind::Beacon
                    && *target_kind != NodeKind::Opinion)
            {
                return true;
            }
            match graph
                .subgraph_from_context(context)
                .meritrank_data
                .graph
                .edge_weight(*target_id, ego_id)
            {
                Ok(Some(_)) => false,
                _ => true,
            }
        })
        .map(|(target_id, _, score, cluster)| (target_id, score, cluster))
        .collect();

    im.sort_by(|(_, a, _), (_, b, _)| b.abs().total_cmp(&a.abs()));

    if prioritize_ego_owned_nodes {
        let mut insert_index = 0;
        for i in 0..im.len() {
            if let Some(owner) = graph.get_object_owner(context, im[i].0) {
                if owner == ego_id {
                    im.swap(i, insert_index);
                    insert_index += 1;
                }
            }
        }
    }

    let index_usize = index as usize;
    let count_usize = count as usize;

    let mut page: Vec<(String, String, Weight, Weight, Cluster, Cluster)> = vec![];
    page.reserve_exact(if count_usize < im.len() { count_usize } else { im.len() });

    for i in index_usize..(index_usize + count_usize) {
        if i >= im.len() {
            break;
        }

        let score_value_of_dst = im[i].1;
        let score_cluster_of_dst = im[i].2;

        let (score_value_of_ego, score_cluster_of_ego) =
            match graph.get_object_owner(context, im[i].0) {
                Some(dst_owner_id) => {
                    graph.fetch_score_cached(context, dst_owner_id, ego_id)
                },
                None => (0.0, 0), 
            };

        page.push((
            ego.to_string(), 
            node_name_from_id(&graph.node_infos, im[i].0),
            score_value_of_dst,
            score_value_of_ego,
            score_cluster_of_dst,
            score_cluster_of_ego,
        ));
    }
    page
}

// --- read_scores ---
pub fn read_scores(
    graph: &mut AugMultiGraph,
    context: &str,
    ego: &str,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
        "{:?} {:?} {:?} {} {} {} {} {} {} {}",
        context, ego, kind_str, hide_personal, score_lt, score_lte, score_gt, score_gte, index, count
    );

    let kind = match kind_from_prefix(kind_str) {
        Ok(x) => x,
        _ => {
            log_error!("Invalid node kind string: {:?}", kind_str);
            return vec![];
        },
    };

    if !graph.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return vec![];
    }

    let ego_id = graph.find_or_add_node_by_name(ego);
    let scores = graph.fetch_all_scores(context, ego_id);

    apply_filters_and_pagination(
        graph, scores, context, ego, ego_id, kind, hide_personal,
        score_lt, score_lte, score_gt, score_gte, index, count, false,
    )
}

// --- read_neighbors ---
pub fn read_neighbors(
    graph: &mut AugMultiGraph,
    context: &str,
    ego: &str,
    focus: &str,
    direction: i64,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
        "{:?} {} {} {} {:?} {} {} {} {} {} {} {}",
        context, ego, focus, direction, kind_str, hide_personal, score_lt, score_lte, score_gt, score_gte, index, count
    );

    let kind = match kind_from_prefix(kind_str) {
        Ok(x) => x,
        _ => {
            log_error!("Invalid node kind string: {:?}", kind_str);
            return vec![];
        },
    };

    let dir = match neighbor_dir_from(direction) {
        Ok(x) => x,
        _ => {
            log_error!("Invalid neighbors direction: {}", direction);
            return vec![];
        },
    };

    let ego_id = graph.find_or_add_node_by_name(ego);
    let focus_id = graph.find_or_add_node_by_name(focus);

    let mut scores = graph.fetch_neighbors(context, ego_id, focus_id, dir);

    if kind == NodeKind::Opinion && direction == NEIGHBORS_INBOUND {
        scores.retain(|&(node_id, _, _)| {
            graph.get_object_owner(context, node_id)
                .map_or(true, |owner_id| owner_id != focus_id)
        });
    }

    apply_filters_and_pagination(
        graph, scores, context, ego, ego_id, kind, hide_personal,
        score_lt, score_lte, score_gt, score_gte, index, count, true,
    )
}

// --- collect_all_edges (helper) ---
pub(crate) fn collect_all_edges(
    graph_mut: &mut AugMultiGraph, 
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>, 
    context: &str,
    ego_id: NodeId,
    index_u32: u32, 
    count_u32: u32, 
) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    let mut edge_ids = Vec::<(NodeId, NodeId, Weight)>::new();
    edge_ids.reserve_exact(indices.len() * 2); 

    log_verbose!("Build final array");

    for (_, src_index) in indices {
        for edge in im_graph.edges(*src_index) {
            if let (Some(src_id), Some(dst_id)) = (ids.get(src_index), ids.get(&edge.target())) {
                let w = *edge.weight();
                if w > -EPSILON && w < EPSILON {
                    log_error!(
                        "Got zero edge weight: {} -> {}",
                        node_name_from_id(&graph_mut.node_infos, *src_id),
                        node_name_from_id(&graph_mut.node_infos, *dst_id)
                    );
                } else {
                    let mut found = false;
                    for (x, y, _) in edge_ids.iter() {
                        if *src_id == *x && *dst_id == *y {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        edge_ids.push((*src_id, *dst_id, w));
                    }
                }
            } else {
                log_error!("Got invalid node index");
            }
        }
    }

    edge_ids.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    edge_ids
        .into_iter()
        .skip(index_u32 as usize)
        .take(count_u32 as usize)
        .map(|(src_id, dst_id, weight_of_dst)| {
            let (score_value_of_dst, score_cluster_of_dst) =
                graph_mut.fetch_score(context, ego_id, dst_id);
            let (score_value_of_ego, score_cluster_of_ego) =
                match graph_mut.get_object_owner(context, dst_id) {
                    Some(dst_owner_id) => {
                        graph_mut.fetch_score_cached(context, dst_owner_id, ego_id)
                    },
                    None => (0.0, 0),
                };

            (
                node_name_from_id(&graph_mut.node_infos, src_id),
                node_name_from_id(&graph_mut.node_infos, dst_id),
                weight_of_dst,
                score_value_of_dst,
                score_value_of_ego,
                score_cluster_of_dst,
                score_cluster_of_ego,
            )
        })
        .collect()
}

// --- read_graph ---
pub fn read_graph(
    graph_mut: &mut AugMultiGraph, 
    context: &str,
    ego: &str,
    focus: &str,
    positive_only: bool,
    index: u32,
    count: u32,
) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    log_command!(
        "{:?} {:?} {:?} {} {} {}",
        context, ego, focus, positive_only, index, count
    );

    if !graph_mut.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return vec![];
    }
    if !graph_mut.node_exists(ego) {
        log_error!("Node does not exist: {:?}", ego);
        return vec![];
    }
    if !graph_mut.node_exists(focus) {
        log_error!("Node does not exist: {:?}", focus);
        return vec![];
    }

    let ego_id = graph_mut.find_or_add_node_by_name(ego);
    let focus_id = graph_mut.find_or_add_node_by_name(focus);
    let force_read_graph_conn = graph_mut.settings.force_read_graph_conn;

    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    let mut im_graph = DiGraph::<NodeId, Weight>::new(); 

    {
        let node_idx = im_graph.add_node(focus_id);
        indices.insert(focus_id, node_idx);
        ids.insert(node_idx, focus_id);
    }

    let node_infos_cloned = graph_mut.node_infos.clone(); 
    let subgraph_data = graph_mut.get_subgraph_from_context(context); 

    log_verbose!("Enumerate focus neighbors");
    let focus_neighbors = subgraph_data.all_outbound_neighbors_normalized(focus_id);

    if ego_id == focus_id {
        log_verbose!("Ego is same as focus");
    } else {
        astar_utils::add_shortest_path_to_graph(
            &subgraph_data, 
            &node_infos_cloned, 
            ego_id,
            focus_id,
            &mut indices,
            &mut ids,
            &mut im_graph,
        );
    }
    if force_read_graph_conn && !indices.contains_key(&ego_id) {
        astar_utils::add_edge_if_valid( 
            &mut im_graph,
            &mut indices,
            &mut ids,
            ego_id,
            focus_id,
            1.0,
        );
    }

    for (dst_id, focus_dst_weight) in focus_neighbors {
        let dst_kind = node_kind_from_id(&node_infos_cloned, dst_id);
        if positive_only && focus_dst_weight <= 0.0 {
            continue;
        }

        if dst_kind == NodeKind::User {
            astar_utils::add_edge_if_valid( 
                &mut im_graph,
                &mut indices,
                &mut ids,
                focus_id,
                dst_id,
                focus_dst_weight,
            );
        } else if dst_kind == NodeKind::Comment || dst_kind == NodeKind::Beacon || dst_kind == NodeKind::Opinion {
            let dst_neighbors = subgraph_data.all_outbound_neighbors_normalized(dst_id);
            for (ngh_id, dst_ngh_weight) in dst_neighbors {
                if (positive_only && dst_ngh_weight <= 0.0)
                    || ngh_id == focus_id
                    || node_kind_from_id(&node_infos_cloned, ngh_id) != NodeKind::User
                {
                    continue;
                }
                let focus_ngh_weight = focus_dst_weight * dst_ngh_weight * if focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 { -1.0 } else { 1.0 };
                astar_utils::add_edge_if_valid( 
                    &mut im_graph,
                    &mut indices,
                    &mut ids,
                    focus_id,
                    ngh_id,
                    focus_ngh_weight,
                );
            }
        }
    }

    log_verbose!("Remove self references");
    for (_, src_index) in indices.iter() {
        let neighbors_to_check: Vec<_> = im_graph.edges(*src_index).map(|edge| (edge.target(), edge.id())).collect();
        for (dst_index, edge_id) in neighbors_to_check {
            if *src_index == dst_index {
                im_graph.remove_edge(edge_id);
            }
        }
    }
    
    collect_all_edges(graph_mut, &indices, &ids, &im_graph, context, ego_id, index, count)
}

// --- read_connected ---
pub fn read_connected(
    graph: &mut AugMultiGraph,
    context: &str,
    ego: &str,
) -> Vec<(String, String)> {
    log_command!("{:?} {:?}", context, ego);

    if !graph.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return vec![];
    }
    if !graph.node_exists(ego) {
        log_error!("Node does not exist: {:?}", ego);
        return vec![];
    }

    let src_id = graph.find_or_add_node_by_name(ego);
    let outgoing_edges: Vec<_> = graph
        .subgraph_from_context(context)
        .meritrank_data
        .graph
        .get_node_data(src_id)
        .unwrap()
        .get_outgoing_edges()
        .collect();

    outgoing_edges
        .into_iter()
        .map(|(dst_id, _)| {
            (ego.to_string(), node_name_from_id(&graph.node_infos, dst_id))
        })
        .collect()
}

// --- read_node_list ---
pub fn read_node_list(graph: &AugMultiGraph) -> Vec<(String,)> {
    log_command!();
    graph
        .node_infos
        .iter()
        .map(|info| (info.name.clone(),))
        .collect()
}

// --- read_edges ---
pub fn read_edges(
    graph: &mut AugMultiGraph, 
    context: &str,
) -> Vec<(String, String, Weight)> {
    log_command!("{:?}", context);

    if !graph.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return vec![];
    }

    let infos_cloned = graph.node_infos.clone(); 
    let mut v: Vec<(String, String, Weight)> = vec![];
    v.reserve(infos_cloned.len() * 2);

    for src_id_usize in 0..infos_cloned.len() {
        let src_id = src_id_usize as NodeId; 
        let src_name = infos_cloned[src_id_usize].name.as_str();

        match graph 
            .subgraph_from_context(context)
            .meritrank_data
            .graph
            .get_node_data(src_id)
        {
            Some(data) => {
                for (dst_id, weight) in data.get_outgoing_edges() {
                    match infos_cloned.get(dst_id as usize) { 
                        Some(x) => v.push((src_name.to_string(), x.name.clone(), weight)),
                        None => log_error!("Node does not exist: {}", dst_id),
                    }
                }
            },
            _ => {},
        };
    }
    v
}

// --- read_mutual_scores ---
pub fn read_mutual_scores(
    graph: &mut AugMultiGraph,
    context: &str,
    ego: &str,
) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", context, ego);

    if !graph.subgraphs.contains_key(context) {
        log_error!("Context does not exist: {:?}", context);
        return vec![];
    }

    let ego_id = graph.find_or_add_node_by_name(ego);
    let ranks = graph.fetch_all_scores(context, ego_id);
    let mut v = Vec::<(String, String, Weight, Weight, Cluster, Cluster)>::new();
    v.reserve_exact(ranks.len());

    for (node_id, score_value_of_dst, score_cluster_of_dst) in ranks {
        let info = if (node_id as usize) < graph.node_infos.len() {
            graph.node_infos[node_id as usize].clone() 
        } else {
            log_error!("Invalid node_id {} encountered in read_mutual_scores for ego {}", node_id, ego);
            NodeInfo { kind: NodeKind::Unknown, name: "".to_string(), seen_nodes: Vec::new() }
        };

        if score_value_of_dst > 0.0 && info.kind == NodeKind::User {
            let (score_value_of_ego, score_cluster_of_ego) =
                match graph.get_object_owner(context, node_id) {
                    Some(dst_owner_id) => {
                        graph.fetch_score_cached(context, dst_owner_id, ego_id)
                    },
                    None => (0.0, 0),
                };

            v.push((
                ego.to_string(),
                info.name,
                score_value_of_dst,
                score_value_of_ego,
                score_cluster_of_dst,
                score_cluster_of_ego,
            ));
        }
    }
    v
}

// --- read_new_edges_filter ---
pub fn read_new_edges_filter(
    graph: &mut AugMultiGraph, 
    src: &str,
) -> Vec<u8> {
    log_command!("{:?}", src);

    if !graph.node_exists(src) {
        log_error!("Node does not exist: {:?}", src);
        return vec![];
    }

    let src_id = graph.find_or_add_node_by_name(src);
    let mut v: Vec<u8> = vec![];
    
    if (src_id as usize) < graph.node_infos.len() {
        v.reserve_exact(graph.node_infos[src_id as usize].seen_nodes.len() * 8);
        for &x_u64 in &graph.node_infos[src_id as usize].seen_nodes {
            for i_byte_idx in 0..8 {
                v.push((x_u64 >> (8 * i_byte_idx)) as u8); 
            }
        }
    } else {
        log_error!("src_id {} is out of bounds for node_infos in read_new_edges_filter", src_id);
    }
    
    v
}
