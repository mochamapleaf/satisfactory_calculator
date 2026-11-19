//! Satisfactory production planner.
//!
//! * `planner` — game data, Xinyu's square incidence-matrix solver (`planner::matrix`) and the
//!   LP generalisation (`planner::lp`) used by the web app.
//! * `graph`, `utilities` — original 2023 resource graph / topological sort code.
//! * `wasm_api` — wasm-bindgen entry points for the React front end.
pub mod graph;
pub mod planner;
pub mod utilities;
pub mod wasm_api;

#[cfg(test)]
mod tests;
