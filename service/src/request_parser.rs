use crate::log::log_error;
use crate::protocol::{
    Command, CMD_RESET, CMD_RECALCULATE_ZERO, CMD_SET_ZERO_OPINION, CMD_RECALCULATE_CLUSTERING,
    CMD_DELETE_EDGE, CMD_DELETE_NODE, CMD_PUT_EDGE, CMD_CREATE_CONTEXT,
    CMD_WRITE_NEW_EDGES_FILTER, CMD_FETCH_NEW_EDGES, CMD_NODE_LIST, CMD_NODE_SCORE,
    CMD_SCORES, CMD_GRAPH, CMD_CONNECTED, CMD_EDGES, CMD_MUTUAL_SCORES,
    CMD_READ_NEW_EDGES_FILTER, CMD_NEIGHBORS,
};
use crate::state_manager::Request;

pub fn request_from_command(command: &Command) -> Request {
    match command.id.as_str() {
        CMD_RESET => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::WriteReset;
            }
        },
        CMD_RECALCULATE_ZERO => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::WriteRecalculateZero;
            }
        },
        CMD_SET_ZERO_OPINION => {
            if let Ok((node, score)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::WriteSetZeroOpinion(
                    command.context.clone(),
                    node,
                    score,
                );
            }
        },
        CMD_RECALCULATE_CLUSTERING => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::WriteRecalculateClustering;
            }
        },
        CMD_DELETE_EDGE => {
            if let Ok((src, dst, index)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::WriteDeleteEdge(
                    command.context.clone(),
                    src,
                    dst,
                    index,
                );
            }
        },
        CMD_DELETE_NODE => {
            if let Ok((node, index)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::WriteDeleteNode(command.context.clone(), node, index);
            }
        },
        CMD_PUT_EDGE => {
            if let Ok((src, dst, amount, index)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::WritePutEdge(
                    command.context.clone(),
                    src,
                    dst,
                    amount,
                    index,
                );
            }
        },
        CMD_CREATE_CONTEXT => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::WriteCreateContext(command.context.clone());
            }
        },
        CMD_WRITE_NEW_EDGES_FILTER => {
            if let Ok((src, filter)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                let v: Vec<u8> = filter;
                return Request::WriteNewEdgesFilter(src, v);
            }
        },
        CMD_FETCH_NEW_EDGES => {
            if let Ok((src, prefix)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::WriteFetchNewEdges(src, prefix);
            }
        },
        CMD_NODE_LIST => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::ReadNodeList;
            }
        },
        CMD_NODE_SCORE => {
            if let Ok((ego, target)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::ReadNodeScore(command.context.clone(), ego, target);
            }
        },
        CMD_SCORES => {
            if let Ok((ego, kind, hide_personal, lt, lte, gt, gte, index, count)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::ReadScores(
                    command.context.clone(),
                    ego,
                    kind,
                    hide_personal,
                    lt,
                    lte,
                    gt,
                    gte,
                    index,
                    count,
                );
            }
        },
        CMD_GRAPH => {
            if let Ok((ego, focus, positive_only, index, count)) =
                rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::ReadGraph(
                    command.context.clone(),
                    ego,
                    focus,
                    positive_only,
                    index,
                    count,
                );
            }
        },
        CMD_CONNECTED => {
            if let Ok(node) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::ReadConnected(command.context.clone(), node);
            }
        },
        CMD_EDGES => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::ReadEdges(command.context.clone());
            }
        },
        CMD_MUTUAL_SCORES => {
            if let Ok(ego) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::ReadMutualScores(command.context.clone(), ego);
            }
        },
        CMD_READ_NEW_EDGES_FILTER => {
            if let Ok(src) = rmp_serde::from_slice(command.payload.as_slice()) {
                return Request::ReadNewEdgesFilter(src);
            }
        },
        CMD_NEIGHBORS => {
            if let Ok((
                ego,
                focus,
                direction,
                kind,
                hide_personal,
                lt,
                lte,
                gt,
                gte,
                index,
                count,
            )) = rmp_serde::from_slice(command.payload.as_slice())
            {
                return Request::ReadNeighbors(
                    command.context.clone(),
                    ego,
                    focus,
                    direction,
                    kind,
                    hide_personal,
                    lt,
                    lte,
                    gt,
                    gte,
                    index,
                    count,
                );
            }
        },
        _ => {
            log_error!("Unexpected command: {:?}", command);
            return Request::None;
        },
    };
    log_error!("Invalid payload for command: {:?}", command);
    Request::None
}
