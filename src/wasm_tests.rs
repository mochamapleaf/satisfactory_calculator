#[cfg(test)]
mod wasm_tests {
    use crate::wasm_calculator::*;
    use crate::wasm_graph::*;
    use wasm_bindgen_test::*;
    use serde_json::json;

    wasm_bindgen_test_configure!(run_in_browser);

    // Sample test data
    fn create_test_game_data() -> String {
        json!({
            "recipes": [
                {
                    "recipe_name": "Iron Ingot",
                    "resources": ["Iron Ore"],
                    "resources_rates": [30.0],
                    "products": ["Iron Ingot"],
                    "product_rates": [30.0],
                    "base_power_consumption": 4.0,
                    "production_method": ["Smelter"],
                    "unlock_tags": ["Tier 0"]
                },
                {
                    "recipe_name": "Iron Plate",
                    "resources": ["Iron Ingot"],
                    "resources_rates": [30.0],
                    "products": ["Iron Plate"],
                    "product_rates": [20.0],
                    "base_power_consumption": 12.0,
                    "production_method": ["Constructor"],
                    "unlock_tags": ["Tier 0"]
                },
                {
                    "recipe_name": "Iron Rod",
                    "resources": ["Iron Ingot"],
                    "resources_rates": [15.0],
                    "products": ["Iron Rod"],
                    "product_rates": [15.0],
                    "base_power_consumption": 4.0,
                    "production_method": ["Constructor"],
                    "unlock_tags": ["Tier 0"]
                },
                {
                    "recipe_name": "Screw",
                    "resources": ["Iron Rod"],
                    "resources_rates": [10.0],
                    "products": ["Screw"],
                    "product_rates": [40.0],
                    "base_power_consumption": 4.0,
                    "production_method": ["Constructor"],
                    "unlock_tags": ["Tier 0"]
                }
            ]
        }).to_string()
    }

    #[wasm_bindgen_test]
    fn test_production_calculator_creation() {
        let game_data = create_test_game_data();
        let calculator = ProductionCalculator::new(&game_data);
        assert!(calculator.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_get_available_items() {
        let game_data = create_test_game_data();
        let calculator = ProductionCalculator::new(&game_data).unwrap();
        
        let items_js = calculator.get_available_items();
        assert!(!items_js.is_null());
        
        // Convert back to Rust to verify
        let items: Vec<String> = serde_wasm_bindgen::from_value(items_js).unwrap();
        assert!(items.contains(&"Iron Ingot".to_string()));
        assert!(items.contains(&"Iron Plate".to_string()));
        assert!(items.contains(&"Iron Rod".to_string()));
        assert!(items.contains(&"Screw".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_validate_targets() {
        let game_data = create_test_game_data();
        let calculator = ProductionCalculator::new(&game_data).unwrap();
        
        let valid_targets = json!([
            {
                "item_name": "Iron Plate",
                "rate": 20.0,
                "unit": "minute"
            }
        ]);
        
        let targets_js = serde_wasm_bindgen::to_value(&valid_targets).unwrap();
        let validation_js = calculator.validate_targets(&targets_js);
        
        let validation: serde_json::Value = serde_wasm_bindgen::from_value(validation_js).unwrap();
        assert_eq!(validation["valid"], true);
    }

    #[wasm_bindgen_test]
    fn test_validate_invalid_targets() {
        let game_data = create_test_game_data();
        let calculator = ProductionCalculator::new(&game_data).unwrap();
        
        let invalid_targets = json!([
            {
                "item_name": "Nonexistent Item",
                "rate": 20.0,
                "unit": "minute"
            }
        ]);
        
        let targets_js = serde_wasm_bindgen::to_value(&invalid_targets).unwrap();
        let validation_js = calculator.validate_targets(&targets_js);
        
        let validation: serde_json::Value = serde_wasm_bindgen::from_value(validation_js).unwrap();
        assert_eq!(validation["valid"], false);
    }

    #[wasm_bindgen_test]
    fn test_calculate_production() {
        let game_data = create_test_game_data();
        let calculator = ProductionCalculator::new(&game_data).unwrap();
        
        let targets = json!([
            {
                "item_name": "Iron Plate",
                "rate": 20.0,
                "unit": "minute"
            }
        ]);
        
        let targets_js = serde_wasm_bindgen::to_value(&targets).unwrap();
        let result_js = calculator.calculate_production(&targets_js);
        
        let result: CalculationResults = serde_wasm_bindgen::from_value(result_js).unwrap();
        assert!(result.success);
        assert!(result.total_power_consumption > 0.0);
        assert!(!result.recipe_rates.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_update_game_data() {
        let initial_data = create_test_game_data();
        let mut calculator = ProductionCalculator::new(&initial_data).unwrap();
        
        let updated_data = json!({
            "recipes": [
                {
                    "recipe_name": "Copper Ingot",
                    "resources": ["Copper Ore"],
                    "resources_rates": [30.0],
                    "products": ["Copper Ingot"],
                    "product_rates": [30.0],
                    "base_power_consumption": 4.0,
                    "production_method": ["Smelter"],
                    "unlock_tags": ["Tier 0"]
                }
            ]
        }).to_string();
        
        let result = calculator.update_game_data(&updated_data);
        assert!(result.is_ok());
        
        // Verify the new data is loaded
        let items_js = calculator.get_available_items();
        let items: Vec<String> = serde_wasm_bindgen::from_value(items_js).unwrap();
        assert!(items.contains(&"Copper Ingot".to_string()));
        assert!(!items.contains(&"Iron Plate".to_string())); // Old data should be gone
    }
}

#[cfg(test)]
mod wasm_graph_tests {
    use crate::wasm_graph::*;

    #[test]
    fn test_wasm_graph_creation() {
        let graph = WasmGraph::new();
        assert!(graph.recipes.is_empty());
        assert!(graph.resources.is_empty());
    }

    #[test]
    fn test_add_recipe() {
        let mut graph = WasmGraph::new();
        
        let recipe = WasmRecipe {
            recipe_name: "Test Recipe".to_string(),
            display_name: None,
            resources: vec!["Input".to_string()],
            resources_rates: vec![10.0],
            products: vec!["Output".to_string()],
            product_rates: vec![5.0],
            base_power_consumption: 4.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&recipe);
        
        assert_eq!(graph.recipes.len(), 1);
        assert!(graph.recipes.contains_key("Test Recipe"));
        assert!(graph.resources.contains_key("Input"));
        assert!(graph.resources.contains_key("Output"));
    }

    #[test]
    fn test_find_all_related() {
        let mut graph = WasmGraph::new();
        
        // Add a simple chain: Iron Ore -> Iron Ingot -> Iron Plate
        let iron_ingot_recipe = WasmRecipe {
            recipe_name: "Iron Ingot".to_string(),
            display_name: None,
            resources: vec!["Iron Ore".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Ingot".to_string()],
            product_rates: vec![30.0],
            base_power_consumption: 4.0,
            production_method: vec!["Smelter".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        let iron_plate_recipe = WasmRecipe {
            recipe_name: "Iron Plate".to_string(),
            display_name: None,
            resources: vec!["Iron Ingot".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Plate".to_string()],
            product_rates: vec![20.0],
            base_power_consumption: 12.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&iron_ingot_recipe);
        graph.add_recipe(&iron_plate_recipe);
        graph.topological_sort();
        
        let targets = vec!["Iron Plate".to_string()];
        let (resources, recipes) = graph.find_all_related(&targets);
        
        assert!(resources.contains(&"Iron Plate".to_string()));
        assert!(resources.contains(&"Iron Ingot".to_string()));
        assert!(resources.contains(&"Iron Ore".to_string()));
        assert!(recipes.contains(&"Iron Plate".to_string()));
        assert!(recipes.contains(&"Iron Ingot".to_string()));
    }

    #[test]
    fn test_construct_matrix() {
        let mut graph = WasmGraph::new();
        
        let recipe = WasmRecipe {
            recipe_name: "Test Recipe".to_string(),
            display_name: None,
            resources: vec!["Input".to_string()],
            resources_rates: vec![10.0],
            products: vec!["Output".to_string()],
            product_rates: vec![5.0],
            base_power_consumption: 4.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&recipe);
        
        let recipes = vec!["Test Recipe".to_string()];
        let resources = vec!["Input".to_string(), "Output".to_string()];
        
        let (matrix, cost_vec) = graph.construct_matrix(&recipes, &resources);
        
        assert_eq!(matrix.nrows(), 1);
        assert_eq!(matrix.ncols(), 2);
        assert_eq!(cost_vec.len(), 1);
        assert_eq!(cost_vec[0], 4.0);
        
        // Check matrix values: should be [-10.0, 5.0] for [Input, Output]
        assert_eq!(matrix[[0, 0]], -10.0); // Input consumption
        assert_eq!(matrix[[0, 1]], 5.0);   // Output production
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = WasmGraph::new();
        
        // Create a simple cycle: A -> B -> A
        let recipe_a_to_b = WasmRecipe {
            recipe_name: "A to B".to_string(),
            display_name: None,
            resources: vec!["A".to_string()],
            resources_rates: vec![10.0],
            products: vec!["B".to_string()],
            product_rates: vec![10.0],
            base_power_consumption: 4.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        let recipe_b_to_a = WasmRecipe {
            recipe_name: "B to A".to_string(),
            display_name: None,
            resources: vec!["B".to_string()],
            resources_rates: vec![5.0],
            products: vec!["A".to_string()],
            product_rates: vec![5.0],
            base_power_consumption: 4.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&recipe_a_to_b);
        graph.add_recipe(&recipe_b_to_a);
        
        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
        
        // Should detect the A-B cycle
        let cycle = &cycles[0];
        assert!(cycle.resources.contains(&"A".to_string()));
        assert!(cycle.resources.contains(&"B".to_string()));
        assert!(cycle.recipes.contains(&"A to B".to_string()));
        assert!(cycle.recipes.contains(&"B to A".to_string()));
    }

    #[test]
    fn test_aluminum_plutonium_cycle() {
        let mut graph = WasmGraph::new();
        
        // Create a simplified but proper cycle: Aluminum Ingot -> Aluminum Casing -> Plutonium Waste -> Aluminum Ingot
        
        // Aluminum Casing recipe (uses Aluminum Ingot)
        let aluminum_casing_recipe = WasmRecipe {
            recipe_name: "Aluminum Casing".to_string(),
            display_name: None,
            resources: vec!["Aluminum Ingot".to_string()],
            resources_rates: vec![90.0],
            products: vec!["Aluminum Casing".to_string()],
            product_rates: vec![60.0],
            base_power_consumption: 62.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 7".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        // Aluminum Casing to Plutonium Waste (intermediate step)
        let aluminum_to_plutonium_recipe = WasmRecipe {
            recipe_name: "Aluminum to Plutonium".to_string(),
            display_name: None,
            resources: vec!["Aluminum Casing".to_string()],
            resources_rates: vec![10.0],
            products: vec!["Plutonium Waste".to_string()],
            product_rates: vec![1.0],
            base_power_consumption: 120.0,
            production_method: vec!["Assembler".to_string()],
            unlock_tags: vec!["Tier 8".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        // Plutonium Waste to Aluminum Ingot (completes the cycle)
        let plutonium_waste_to_aluminum_recipe = WasmRecipe {
            recipe_name: "Instant Scrap".to_string(),
            display_name: None,
            resources: vec!["Plutonium Waste".to_string()],
            resources_rates: vec![1.0],
            products: vec!["Aluminum Ingot".to_string()],
            product_rates: vec![15.0],
            base_power_consumption: 300.0,
            production_method: vec!["Particle Accelerator".to_string()],
            unlock_tags: vec!["Tier 8".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&aluminum_casing_recipe);
        graph.add_recipe(&aluminum_to_plutonium_recipe);
        graph.add_recipe(&plutonium_waste_to_aluminum_recipe);
        
        // Test cycle detection directly
        let cycles = graph.detect_cycles();
        println!("Direct cycle detection: {:?}", cycles);
        
        // Test dependency resolution with cycles
        let targets = vec!["Aluminum Casing".to_string()];
        let resolution = graph.resolve_dependencies_with_cycles(&targets);
        
        // Debug output
        println!("Cycles detected: {:?}", resolution.cycles);
        println!("Has cycles: {}", resolution.has_cycles);
        println!("Resources: {:?}", resolution.resources);
        println!("Recipes: {:?}", resolution.recipes);
        
        // Now we should have a proper cycle
        assert!(resolution.has_cycles);
        assert!(!resolution.cycles.is_empty());
        
        // Should include all the recipes in the cycle
        assert!(resolution.recipes.contains(&"Aluminum Casing".to_string()));
        assert!(resolution.recipes.contains(&"Aluminum to Plutonium".to_string()));
        assert!(resolution.recipes.contains(&"Instant Scrap".to_string()));
        
        // Test matrix construction with cycles
        let (matrix, cost_vec, cycles) = graph.construct_matrix_with_cycles(&resolution.recipes, &resolution.resources);
        
        assert!(!cycles.is_empty());
        assert_eq!(matrix.nrows(), resolution.recipes.len());
        assert_eq!(matrix.ncols(), resolution.resources.len());
        assert_eq!(cost_vec.len(), resolution.recipes.len());
        
        // The matrix should be well-formed even with cycles
        assert!(matrix.nrows() > 0);
        assert!(matrix.ncols() > 0);
    }

    #[test]
    fn test_dependency_resolution_without_cycles() {
        let mut graph = WasmGraph::new();
        
        // Create a simple linear chain: Iron Ore -> Iron Ingot -> Iron Plate
        let iron_ingot_recipe = WasmRecipe {
            recipe_name: "Iron Ingot".to_string(),
            display_name: None,
            resources: vec!["Iron Ore".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Ingot".to_string()],
            product_rates: vec![30.0],
            base_power_consumption: 4.0,
            production_method: vec!["Smelter".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        let iron_plate_recipe = WasmRecipe {
            recipe_name: "Iron Plate".to_string(),
            display_name: None,
            resources: vec!["Iron Ingot".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Plate".to_string()],
            product_rates: vec![20.0],
            base_power_consumption: 12.0,
            production_method: vec!["Constructor".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        graph.add_recipe(&iron_ingot_recipe);
        graph.add_recipe(&iron_plate_recipe);
        
        let targets = vec!["Iron Plate".to_string()];
        let resolution = graph.resolve_dependencies_with_cycles(&targets);
        
        assert!(!resolution.has_cycles);
        assert!(resolution.cycles.is_empty());
        assert!(resolution.recipes.contains(&"Iron Plate".to_string()));
        assert!(resolution.recipes.contains(&"Iron Ingot".to_string()));
    }
}