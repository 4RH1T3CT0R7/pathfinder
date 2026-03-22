use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    pub steps_taken: u32,
    pub cells_visited: u32,
    pub path_length: u32,
    pub dead_ends: u32,
    pub frontier_max_size: u32,
    pub elapsed_us: u64,
}
