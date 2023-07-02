#[cfg(target_arch = "wasm32")]
mod yew_frontend;
#[cfg(target_arch = "wasm32")]
use yew_frontend::*;

mod graph;
mod utilities;

#[cfg(target_arch = "wasm32")]
fn main(){
    yew::Renderer::<App>::new().render();
}

#[cfg(not(target_arch = "wasm32"))]
fn main(){
    let mut target_map = std::collections::HashMap::<String, f64>::new();
    target_map.insert("Desc_HeavyOilResidue_C".to_string(), 300.0);
    const DEFAULT_JSON: &str = include_str!("../recipes/auto_output.json");
    let mut inst = crate::graph::Graph::from_str(DEFAULT_JSON);
    let (mut resources, recipes) = inst.find_all_related(target_map.keys().map(|s| s.as_str()));
    let (mut matrix,mut cost_vec) = inst.construct_matrix(&recipes, &resources);

    let col_num = matrix.ncols();
    let col_selector: Vec<_> = (1..col_num).collect();
    let matrix = matrix.select(ndarray::Axis(1), &col_selector); //remove empty column "world root"
    let mut matrix_A = matrix.t().to_owned();
    resources.remove(0);
    let mut target_vals : Vec<f64>= resources.iter()
        .map(|r| *target_map.get(r).unwrap_or(&0_f64)).collect();
    let (solution, objective) = crate::utilities::solve_lp(&matrix_A, &cost_vec,&target_vals, minilp::ComparisonOp::Ge);
    let temp: Vec<_> = solution.iter().zip(recipes.iter()).filter(|(&s,_)| s > 0_f64 ).collect();
    let output_recipes: Vec<String> = temp.iter().map(|(_, s)| (*s).clone()).collect();
    let output_quantites : Vec<f64>= temp.iter().map(|(&v, _)| v).collect();
    println!("Total power used: {}", objective);
    for i in 0..output_quantites.len(){
        println!("{}: {}", output_recipes[i], output_quantites[i]);
    }
}
