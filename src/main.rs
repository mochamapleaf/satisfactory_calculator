#[cfg(target_arch = "wasm32")]
mod yew_frontend;
#[cfg(target_arch = "wasm32")]
use yew_frontend::*;

mod graph;
mod utilities;

#[cfg(target_arch = "wasm32")]
fn main() {
    yew::Renderer::<App>::new().render();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut target_map = std::collections::HashMap::<String, f64>::new();
    target_map.insert("Plastic".to_string(), 300.0);
    const DEFAULT_JSON: &str = include_str!("../recipes/auto_output.json");
    let mut inst = crate::graph::Graph::from_str(DEFAULT_JSON);
    let (mut resources, recipes) = inst.find_all_related(target_map.keys().map(|s| s.as_str()));
    let (mut matrix, mut cost_vec) = inst.construct_matrix(&recipes, &resources);
    let col_num = matrix.ncols();
    let root_pos = resources
        .iter()
        .position(|v| v == graph::WORLD_ROOT)
        .unwrap();
    let mut col_selector: Vec<_> = (0..col_num).collect();
    col_selector.remove(root_pos);
    resources.remove(root_pos);
    let matrix = matrix.select(ndarray::Axis(1), &col_selector); //remove empty column "world root"
    let mut matrix_A = matrix.t().to_owned();
    let mut target_vals: Vec<f64> = resources
        .iter()
        .map(|r| *target_map.get(r).unwrap_or(&0_f64))
        .collect();
    let (solution, objective) =
        crate::utilities::solve_lp(&matrix_A, &cost_vec, &target_vals, minilp::ComparisonOp::Ge);
    let temp: Vec<_> = solution
        .iter()
        .zip(recipes.iter())
        .filter(|(&s, _)| s > 0_f64)
        .collect();
    const THRESHOLD_VAL: f64 = f64::EPSILON * 1000.0;
    let mut output: Vec<(String, f64)> = temp
        .iter()
        .filter(|(&v, _)| v > THRESHOLD_VAL)
        .map(|(&v, s)| ((*s).clone(), v))
        .collect();
    let mut temp_graph = inst.select(output.iter().map(|(k, _)| k) );
    output.sort_unstable_by_key(|(k, _)| inst.recipes[k].resources.iter().map(|v| temp_graph.topological_sort_result[v].1).min().unwrap_or(u64::MAX));
    output.reverse();
    println!("{:?}", matrix_A);
    println!("Total power used: {}", objective);
    for i in 0..output.len() {
        println!("{}: {}", output[i].0, output[i].1);
    }
}
