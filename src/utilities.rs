use md5::{Digest, Md5};
use minilp::{OptimizationDirection, Problem};
use ndarray::Array2;

pub fn solve_lp(
    matrix: &Array2<f64>,
    cost_vec: &Vec<f64>,
    target_vec: &Vec<f64>,
    lp_option: minilp::ComparisonOp,
) -> (Vec<f64>, f64) {
    let mut problem = Problem::new(OptimizationDirection::Minimize);
    let vars: Vec<_> = cost_vec
        .iter()
        .map(|&v| problem.add_var(v, (0.0, f64::INFINITY)))
        .collect(); //each var >= 0
    for row in 0..matrix.nrows() {
        let mut row_expr = minilp::LinearExpr::empty();
        matrix
            .row(row)
            .iter()
            .enumerate()
            .for_each(|(i, &v)| row_expr.add(vars[i], v));
        problem.add_constraint(row_expr, lp_option, target_vec[row]);
    }
    let solution = problem.solve().unwrap();
    (
        (0..matrix.ncols())
            .map(|i| *solution.var_value(vars[i]))
            .collect(),
        solution.objective(),
    )
}

pub fn generate_item_image_link(name: &str) -> String {
    let pic_name = format!("{}.png", name).replace(" ", "_");
    let mut hasher = Md5::new();
    hasher.update(pic_name.as_bytes());
    let hex_str = format!("{:x}", hasher.finalize()).into_bytes();
    format!(
        "https://satisfactory.wiki.gg/images/{0}/{0}{1}/{2}",
        hex_str[0] as char, hex_str[1] as char, pic_name
    )
}
