use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use std::collections::HashMap;
use crate::wasm_graph::{WasmGraph, GameData, ProductionTarget, CalculationResults};
use crate::utilities::solve_lp;

// WASM-compatible ProductionCalculator
#[wasm_bindgen]
pub struct ProductionCalculator {
    graph: WasmGraph,
    game_data: Option<GameData>,
}

// Settings for calculations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CalculationSettings {
    pub enabled_recipes: Option<Vec<String>>,
    pub disabled_recipes: Option<Vec<String>>,
    pub optimization_target: String, // "power", "machines", "resources"
}

impl Default for CalculationSettings {
    fn default() -> Self {
        Self {
            enabled_recipes: None,
            disabled_recipes: None,
            optimization_target: "power".to_string(),
        }
    }
}

#[wasm_bindgen]
impl ProductionCalculator {
    /// Create a new ProductionCalculator with game data
    #[wasm_bindgen(constructor)]
    pub fn new(game_data_json: &str) -> Result<ProductionCalculator, JsValue> {
        // Set up console error panic hook for better debugging
        console_error_panic_hook::set_once();
        
        let game_data: GameData = serde_json::from_str(game_data_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse game data: {}", e)))?;
        
        let mut graph = WasmGraph::new();
        
        // Add all recipes to the graph
        for recipe in &game_data.recipes {
            graph.add_recipe(recipe);
        }
        
        graph.topological_sort();
        
        Ok(ProductionCalculator {
            graph,
            game_data: Some(game_data),
        })
    }

    /// Update game data from JSON string
    #[wasm_bindgen]
    pub fn update_game_data(&mut self, game_data_json: &str) -> Result<(), JsValue> {
        let game_data: GameData = serde_json::from_str(game_data_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse game data: {}", e)))?;
        
        // Rebuild graph with new data
        let mut new_graph = WasmGraph::new();
        for recipe in &game_data.recipes {
            new_graph.add_recipe(recipe);
        }
        new_graph.topological_sort();
        
        self.graph = new_graph;
        self.game_data = Some(game_data);
        
        Ok(())
    }

    /// Calculate production requirements for given targets
    #[wasm_bindgen]
    pub fn calculate_production(&self, targets_js: &JsValue) -> JsValue {
        let calculation_result = self.calculate_production_internal(targets_js);
        
        match serde_wasm_bindgen::to_value(&calculation_result) {
            Ok(js_value) => js_value,
            Err(e) => {
                let error_result = CalculationResults {
                    recipe_rates: HashMap::new(),
                    resource_flows: HashMap::new(),
                    total_power_consumption: 0.0,
                    success: false,
                    error_message: Some(format!("Serialization error: {}", e)),
                };
                serde_wasm_bindgen::to_value(&error_result).unwrap_or(JsValue::NULL)
            }
        }
    }

    /// Update calculation settings
    #[wasm_bindgen]
    pub fn update_settings(&mut self, settings_js: &JsValue) {
        if let Ok(_settings) = serde_wasm_bindgen::from_value::<CalculationSettings>(settings_js.clone()) {
            // Settings updated successfully
            // For now, we'll store settings in future iterations
        }
    }

    /// Get list of available items
    #[wasm_bindgen]
    pub fn get_available_items(&self) -> JsValue {
        let mut items = Vec::new();
        
        // Collect all unique products from recipes
        for recipe in self.graph.recipes.values() {
            for product in &recipe.products {
                if !items.contains(product) {
                    items.push(product.clone());
                }
            }
        }
        
        items.sort();
        
        match serde_wasm_bindgen::to_value(&items) {
            Ok(js_value) => js_value,
            Err(_) => JsValue::NULL,
        }
    }

    /// Validate production targets
    #[wasm_bindgen]
    pub fn validate_targets(&self, targets_js: &JsValue) -> JsValue {
        let validation_result = match serde_wasm_bindgen::from_value::<Vec<ProductionTarget>>(targets_js.clone()) {
            Ok(targets) => {
                let mut errors = Vec::new();
                
                for target in &targets {
                    // Check if item exists in any recipe output
                    let item_exists = self.graph.recipes.values()
                        .any(|recipe| recipe.products.contains(&target.item_name));
                    
                    if !item_exists {
                        errors.push(format!("Item '{}' not found in available recipes", target.item_name));
                    }
                    
                    if target.rate <= 0.0 {
                        errors.push(format!("Production rate for '{}' must be positive", target.item_name));
                    }
                }
                
                serde_json::json!({
                    "valid": errors.is_empty(),
                    "errors": errors
                })
            }
            Err(e) => {
                serde_json::json!({
                    "valid": false,
                    "errors": [format!("Invalid target format: {}", e)]
                })
            }
        };
        
        match serde_wasm_bindgen::to_value(&validation_result) {
            Ok(js_value) => js_value,
            Err(_) => JsValue::NULL,
        }
    }

    /// Apply recipe preset (enable/disable specific recipes)
    #[wasm_bindgen]
    pub fn apply_recipe_preset(&mut self, preset_js: &JsValue) {
        if let Ok(_settings) = serde_wasm_bindgen::from_value::<CalculationSettings>(preset_js.clone()) {
            // For now, we'll implement this in future iterations
            // This would filter the graph to only include enabled recipes
        }
    }
}

impl ProductionCalculator {
    /// Internal calculation method
    fn calculate_production_internal(&self, targets_js: &JsValue) -> CalculationResults {
        // Parse targets from JavaScript
        let targets = match serde_wasm_bindgen::from_value::<Vec<ProductionTarget>>(targets_js.clone()) {
            Ok(targets) => targets,
            Err(e) => {
                return CalculationResults {
                    recipe_rates: HashMap::new(),
                    resource_flows: HashMap::new(),
                    total_power_consumption: 0.0,
                    success: false,
                    error_message: Some(format!("Failed to parse targets: {}", e)),
                };
            }
        };

        if targets.is_empty() {
            return CalculationResults {
                recipe_rates: HashMap::new(),
                resource_flows: HashMap::new(),
                total_power_consumption: 0.0,
                success: false,
                error_message: Some("No production targets specified".to_string()),
            };
        }

        // Convert targets to target resource list
        let target_items: Vec<String> = targets.iter().map(|t| t.item_name.clone()).collect();
        
        // Find all related resources and recipes
        let (resources, recipes) = self.graph.find_all_related(&target_items);
        
        if recipes.is_empty() {
            return CalculationResults {
                recipe_rates: HashMap::new(),
                resource_flows: HashMap::new(),
                total_power_consumption: 0.0,
                success: false,
                error_message: Some("No recipes found for target items".to_string()),
            };
        }

        // Construct matrix for linear programming
        let (matrix, cost_vec) = self.graph.construct_matrix(&recipes, &resources);
        
        if matrix.nrows() == 0 || matrix.ncols() == 0 {
            return CalculationResults {
                recipe_rates: HashMap::new(),
                resource_flows: HashMap::new(),
                total_power_consumption: 0.0,
                success: false,
                error_message: Some("Empty matrix constructed".to_string()),
            };
        }

        // Remove world root column if present
        let world_root_pos = resources.iter().position(|v| v == crate::wasm_graph::WORLD_ROOT);
        let (final_matrix, final_resources) = if let Some(root_pos) = world_root_pos {
            let mut col_selector: Vec<_> = (0..matrix.ncols()).collect();
            col_selector.remove(root_pos);
            let mut filtered_resources = resources.clone();
            filtered_resources.remove(root_pos);
            (matrix.select(ndarray::Axis(1), &col_selector), filtered_resources)
        } else {
            (matrix, resources)
        };

        // Create target vector
        let mut target_map = HashMap::new();
        for target in &targets {
            // Convert rate to per-minute if needed
            let rate_per_minute = match target.unit.as_str() {
                "second" => target.rate * 60.0,
                "hour" => target.rate / 60.0,
                _ => target.rate, // assume per minute
            };
            target_map.insert(target.item_name.clone(), rate_per_minute);
        }

        let target_vals: Vec<f64> = final_resources
            .iter()
            .map(|r| *target_map.get(r).unwrap_or(&0.0))
            .collect();

        // Solve linear programming problem
        let matrix_a = final_matrix.t().to_owned();
        
        match solve_lp(&matrix_a, &cost_vec, &target_vals, minilp::ComparisonOp::Ge) {
            (solution, objective) => {
                // Filter out near-zero solutions
                const THRESHOLD_VAL: f64 = f64::EPSILON * 1000.0;
                let mut recipe_rates = HashMap::new();
                let mut resource_flows = HashMap::new();

                // Process recipe rates
                for (i, &rate) in solution.iter().enumerate() {
                    if rate > THRESHOLD_VAL {
                        if let Some(recipe_name) = recipes.get(i) {
                            recipe_rates.insert(recipe_name.clone(), rate);
                        }
                    }
                }

                // Calculate resource flows
                for (_i, resource_name) in final_resources.iter().enumerate() {
                    let mut net_flow = 0.0;
                    
                    // Sum up production and consumption for this resource
                    for (j, &recipe_rate) in solution.iter().enumerate() {
                        if recipe_rate > THRESHOLD_VAL {
                            if let Some(recipe_name) = recipes.get(j) {
                                if let Some(recipe) = self.graph.recipes.get(recipe_name) {
                                    // Add production
                                    for (k, product) in recipe.products.iter().enumerate() {
                                        if product == resource_name {
                                            net_flow += recipe.product_rates[k] * recipe_rate;
                                        }
                                    }
                                    
                                    // Subtract consumption
                                    for (k, resource) in recipe.resources.iter().enumerate() {
                                        if resource == resource_name {
                                            net_flow -= recipe.resources_rates[k] * recipe_rate;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if net_flow.abs() > THRESHOLD_VAL {
                        resource_flows.insert(resource_name.clone(), net_flow);
                    }
                }

                CalculationResults {
                    recipe_rates,
                    resource_flows,
                    total_power_consumption: objective,
                    success: true,
                    error_message: None,
                }
            }
        }
    }
}

// Export the calculator for JavaScript use
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}