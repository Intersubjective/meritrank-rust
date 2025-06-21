use bincode::{Decode, Encode};
use crate::aug_graph::NodeName;
use crate::aug_graph::read::FilterOptions;
use meritrank_core::Weight;
use crate::aug_graph::vsids::Magnitude;

#[derive(Debug, Encode, Decode)]
pub struct Response {
    pub response: u64,
}

pub type SubgraphName = String;

#[derive(Debug, Encode, Decode)]
pub enum Request {
    ReadScoresReq {
        subgraph_name: SubgraphName,
        ego: NodeName,
        score_options: FilterOptions,
    },
    WriteEdgeReq {
        subgraph_name: SubgraphName,
        src: NodeName,
        dst: NodeName,
        amount: Weight,
        magnitude: Magnitude,
    },
}