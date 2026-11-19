//! Xinyu's original incidence-matrix method (readme.md):
//! rows = recipes, columns = items; add one `[Input]` row per raw resource so that
//! Aᵀ is square, then x = (Aᵀ)⁻¹ b.
//!
//! This is exact and fast when every non-raw item has exactly one producing recipe and
//! there are no byproducts. With several recipes per item, byproducts or cycles that
//! make Aᵀ non-square or singular, `solve_square` returns an error and the LP
//! (`super::lp`) takes over.
use super::data::GameData;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixError {
    NotSquare { rows: usize, cols: usize },
    Singular,
    UnknownRecipe(String),
}

/// Result of the square-matrix solve.
#[derive(Debug, Clone)]
pub struct SquareSolution {
    /// Machines (100 % clock) per recipe id.
    pub machines: Vec<(String, f64)>,
    /// Raw input per minute per item id.
    pub inputs: Vec<(String, f64)>,
}

/// Build the incidence matrix A (rows = recipes then input rows, cols = items).
pub fn incidence_matrix(
    data: &GameData,
    recipe_ids: &[String],
    items: &[String],
    input_items: &[String],
) -> Result<Vec<Vec<f64>>, MatrixError> {
    let col: HashMap<&str, usize> = items.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let mut a = Vec::new();
    for rid in recipe_ids {
        let r = data.recipe(rid).ok_or_else(|| MatrixError::UnknownRecipe(rid.clone()))?;
        let mut row = vec![0.0; items.len()];
        for (it, q) in &r.inputs {
            row[col[it.as_str()]] -= q;
        }
        for (it, q) in &r.outputs {
            row[col[it.as_str()]] += q;
        }
        a.push(row);
    }
    for it in input_items {
        let mut row = vec![0.0; items.len()];
        row[col[it.as_str()]] = 1.0;
        a.push(row);
    }
    Ok(a)
}

/// Solve Aᵀx = b for a chain where each item has exactly one recipe.
/// `targets` are item id → net output per minute.
pub fn solve_square(
    data: &GameData,
    recipe_ids: &[String],
    targets: &[(String, f64)],
) -> Result<SquareSolution, MatrixError> {
    // collect items in a stable order
    let mut items: Vec<String> = Vec::new();
    let mut produced = std::collections::HashSet::new();
    for rid in recipe_ids {
        let r = data.recipe(rid).ok_or_else(|| MatrixError::UnknownRecipe(rid.clone()))?;
        for (it, _) in r.inputs.iter().chain(r.outputs.iter()) {
            if !items.contains(it) {
                items.push(it.clone());
            }
        }
        for (it, _) in &r.outputs {
            produced.insert(it.clone());
        }
    }
    for (t, _) in targets {
        if !items.contains(t) {
            items.push(t.clone());
        }
    }
    let input_items: Vec<String> = items.iter().filter(|i| !produced.contains(*i)).cloned().collect();
    let a = incidence_matrix(data, recipe_ids, &items, &input_items)?;
    let n = items.len();
    if a.len() != n {
        return Err(MatrixError::NotSquare { rows: a.len(), cols: n });
    }
    // Aᵀ x = b
    let mut m: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| a[j][i]).collect()).collect();
    let mut b = vec![0.0; n];
    for (t, q) in targets {
        let i = items.iter().position(|x| x == t).unwrap();
        b[i] = *q;
    }
    let x = gauss_solve(&mut m, &mut b).ok_or(MatrixError::Singular)?;
    let k = recipe_ids.len();
    Ok(SquareSolution {
        machines: recipe_ids.iter().cloned().zip(x[..k].iter().cloned()).collect(),
        // input rows were +1 on the item, and the balance is Σ = b, so the input
        // variable is the (positive) amount supplied.
        inputs: input_items.into_iter().zip(x[k..].iter().cloned()).collect(),
    })
}

/// Gaussian elimination with partial pivoting.
pub fn gauss_solve(m: &mut [Vec<f64>], b: &mut [f64]) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| m[i][c].abs().partial_cmp(&m[j][c].abs()).unwrap())?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        b.swap(c, p);
        for r in 0..n {
            if r != c {
                let f = m[r][c] / m[c][c];
                if f != 0.0 {
                    for k in c..n {
                        m[r][k] -= f * m[c][k];
                    }
                    b[r] -= f * b[c];
                }
            }
        }
    }
    Some((0..n).map(|i| b[i] / m[i][i]).collect())
}
