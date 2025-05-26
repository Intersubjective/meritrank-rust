// Existing pub mods from refactoring
pub mod astar_utils;
pub mod read_ops;
pub mod write_ops;
pub mod settings_parser;
pub mod request_parser;

// Original lib.rs pub mods
pub mod log;
pub mod protocol;

// Add pub mods for other core library components
pub mod astar;
pub mod aug_multi_graph;
pub mod bloom_filter;
pub mod constants;
pub mod nodes;
pub mod operations;
pub mod quantiles;
pub mod request_handler;
pub mod state_manager;
pub mod subgraph;
pub mod vsids;
pub mod zero_opinion;

// Test modules (conditionally compiled)
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub mod test_data;
