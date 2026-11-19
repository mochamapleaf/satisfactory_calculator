use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use std::collections::HashMap;
use std::collections::HashSet;
use ndarray::Array2;

// WASM-compatible Recipe structure with TypeScript bindings
#[derive(Debug, Deserialize, Serialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmRecipe {
    pub recipe_name: String,
    pub display_name: Option<String>,
    pub resources: Vec<String>,
    pub resources_rates: Vec<f64>,
    pub products: Vec<String>,
    pub product_rates: Vec<f64>,
    pub base_power_consumption: f64,
    pub production_method: Vec<String>,
    pub unlock_tags: Vec<String>,
    pub is_alternate: Option<bool>,
    pub default_machine_type: Option<String>,
    pub production_time: Option<f64>,
    pub allows_machine_selection: Option<bool>,
    pub compatible_machines: Option<Vec<String>>,
}

// WASM-compatible ResourceNode structure
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmResourceNode {
    pub resource_name: String,
    pub ingress_edges: Vec<WasmEdge>,
    pub egress_edges: Vec<WasmEdge>,
}

// WASM-compatible Edge structure
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmEdge {
    pub from: String,
    pub to: String,
    pub recipe_name: String,
}

// WASM-compatible Graph structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WasmGraph {
    pub recipes: HashMap<String, WasmRecipe>,
    pub resources: HashMap<String, WasmResourceNode>,
    pub topological_sort_result: HashMap<String, (u64, u64)>,
}

// Production target for calculations
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ProductionTarget {
    pub item_name: String,
    pub rate: f64,
    pub unit: String, // "minute", "hour", "second"
}

// Game data structure for JSON loading
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GameData {
    pub items: Option<HashMap<String, serde_json::Value>>,
    pub recipes: Vec<WasmRecipe>,
    pub machines: Option<HashMap<String, serde_json::Value>>,
}

// Calculation results structure
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CalculationResults {
    pub recipe_rates: HashMap<String, f64>,
    pub resource_flows: HashMap<String, f64>,
    pub total_power_consumption: f64,
    pub success: bool,
    pub error_message: Option<String>,
}

// Cycle information structure
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CycleInfo {
    pub resources: Vec<String>,
    pub recipes: Vec<String>,
    pub can_optimize_as_super_node: bool,
}

// Dependency resolution result
#[derive(Debug, Serialize, Deserialize, Clone, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct DependencyResolution {
    pub resources: Vec<String>,
    pub recipes: Vec<String>,
    pub cycles: Vec<CycleInfo>,
    pub has_cycles: bool,
}

pub static WORLD_ROOT: &str = "world_root";

impl Default for WasmGraph {
    fn default() -> Self {
        Self {
            recipes: HashMap::new(),
            resources: HashMap::new(),
            topological_sort_result: HashMap::new(),
        }
    }
}

impl WasmGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Create graph from JSON string containing recipes
    pub fn from_json_str(json_data: &str) -> Result<Self, String> {
        let game_data: GameData = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let mut graph = Self::new();
        
        for recipe in game_data.recipes {
            graph.add_recipe(&recipe);
        }
        
        graph.topological_sort();
        Ok(graph)
    }

    /// Add a recipe to the graph
    pub fn add_recipe(&mut self, recipe: &WasmRecipe) {
        let recipe_name = &recipe.recipe_name;
        self.recipes.insert(recipe_name.clone(), recipe.clone());
        
        for product in recipe.products.iter() {
            self.add_resource_node(product);
            
            let mut process_resources = recipe.resources.clone();
            if process_resources.is_empty() {
                // The resource comes from the world directly
                process_resources.push(WORLD_ROOT.to_string());
            }
            
            for resource in process_resources.iter() {
                self.add_resource_node(resource);
                
                let new_edge = WasmEdge {
                    from: resource.clone(),
                    to: product.clone(),
                    recipe_name: recipe_name.clone(),
                };
                
                // Add edge to resource's egress edges
                if let Some(from_node) = self.resources.get_mut(resource) {
                    from_node.egress_edges.push(new_edge.clone());
                }
                
                // Add edge to product's ingress edges (if different from resource)
                if resource != product {
                    if let Some(to_node) = self.resources.get_mut(product) {
                        to_node.ingress_edges.push(new_edge);
                    }
                } else {
                    // Self-loop case
                    if let Some(node) = self.resources.get_mut(resource) {
                        node.ingress_edges.push(new_edge);
                    }
                }
            }
        }
    }

    /// Add a resource node if it doesn't exist
    fn add_resource_node(&mut self, resource_name: &str) -> bool {
        if self.resources.contains_key(resource_name) {
            return false;
        }
        
        self.resources.insert(
            resource_name.to_string(),
            WasmResourceNode {
                resource_name: resource_name.to_string(),
                ingress_edges: Vec::new(),
                egress_edges: Vec::new(),
            },
        );
        true
    }

    /// Perform topological sort on the graph
    pub fn topological_sort(&mut self) {
        let mut pending: Vec<String> = self.resources.keys().cloned().collect();
        pending.sort_unstable(); // Ensure consistent results
        
        let mut topology: HashMap<String, (u64, u64)> = HashMap::new();
        let mut cur = 0_u64;
        
        while !pending.is_empty() {
            let next = pending.pop().unwrap();
            let mut dfs_stack: Vec<String> = Vec::new();
            dfs_stack.push(next);
            
            while let Some(s) = dfs_stack.pop() {
                if !topology.contains_key(&s) {
                    // First time being discovered
                    topology.insert(s.clone(), (cur, u64::MAX));
                    cur += 1;
                    dfs_stack.push(s.clone()); // Add it back for second discovery
                    
                    // Remove from pending
                    if let Some(i) = pending.iter().position(|v| v == &s) {
                        pending.remove(i);
                    }
                    
                    // Add egress edges to stack
                    if let Some(resource_node) = self.resources.get(&s) {
                        for egress in &resource_node.egress_edges {
                            if !topology.contains_key(&egress.to) {
                                dfs_stack.push(egress.to.clone());
                            }
                        }
                    }
                } else {
                    // Second time being discovered
                    if let Some(entry) = topology.get_mut(&s) {
                        entry.1 = cur;
                        cur += 1;
                    }
                }
            }
        }
        
        self.topological_sort_result = topology;
    }

    /// Find all resources that can be produced with given available resources
    pub fn expand_coverage(&self, available_resources: &mut HashSet<String>) {
        loop {
            let mut next_iter = false;
            let cur_sources = available_resources.clone();
            
            for source in cur_sources.iter() {
                if let Some(resource_node) = self.resources.get(source) {
                    for egress in &resource_node.egress_edges {
                        if available_resources.contains(&egress.to) {
                            continue;
                        }
                        
                        let mut condition_fulfilled = true;
                        if let Some(recipe) = self.recipes.get(&egress.recipe_name) {
                            for requirement in &recipe.resources {
                                if !available_resources.contains(requirement) {
                                    condition_fulfilled = false;
                                    break;
                                }
                            }
                        }
                        
                        if condition_fulfilled {
                            next_iter = true;
                            available_resources.insert(egress.to.clone());
                        }
                    }
                }
            }
            
            if !next_iter {
                break;
            }
        }
    }

    /// Find all related recipes and resources for given target resources
    /// Enhanced version that handles circular dependencies without requiring topological sort
    pub fn find_all_related(&self, target_resources: &[String]) -> (Vec<String>, Vec<String>) {
        let mut pending: Vec<String> = target_resources.to_vec();
        let mut processed = HashSet::<String>::new();
        let mut related_recipes = HashSet::<String>::new();
        
        while let Some(cur_item) = pending.pop() {
            if processed.contains(&cur_item) {
                continue;
            }
            
            // Push ingredients
            if let Some(resource_node) = self.resources.get(&cur_item) {
                for edge in &resource_node.ingress_edges {
                    pending.push(edge.from.clone());
                    let is_new = related_recipes.insert(edge.recipe_name.clone());
                    
                    // If the pushed recipe contains byproducts, push them as well
                    if is_new {
                        if let Some(recipe) = self.recipes.get(&edge.recipe_name) {
                            for byproduct in recipe.products.iter().filter(|s| **s != cur_item) {
                                pending.push(byproduct.clone());
                            }
                        }
                    }
                }
            }
            
            processed.insert(cur_item);
        }
        
        let mut resource_vec: Vec<String> = processed.into_iter().collect();
        let mut recipes_vec: Vec<String> = related_recipes.into_iter().collect();
        
        // Sort resources by topological order
        resource_vec.sort_by_key(|v| {
            self.topological_sort_result
                .get(v)
                .map(|(_, finish)| u64::MAX - finish)
                .unwrap_or(0)
        });
        
        recipes_vec.sort();
        (resource_vec, recipes_vec)
    }

    /// Construct matrix for linear programming
    pub fn construct_matrix(
        &self,
        recipes: &[String],
        resources: &[String],
    ) -> (Array2<f64>, Vec<f64>) {
        let mut matrix: Vec<Vec<f64>> = Vec::new();
        let mut cost_vec: Vec<f64> = Vec::new();
        
        for recipe_name in recipes.iter() {
            if let Some(recipe) = self.recipes.get(recipe_name) {
                let mut new_row = vec![0.0; resources.len()];
                
                // Add negative weights for resources (inputs)
                for (i, resource) in recipe.resources.iter().enumerate() {
                    if let Some(target_index) = resources.iter().position(|v| v == resource) {
                        new_row[target_index] -= recipe.resources_rates[i];
                    }
                }
                
                // Add positive weights for products (outputs)
                for (i, product) in recipe.products.iter().enumerate() {
                    if let Some(target_index) = resources.iter().position(|v| v == product) {
                        new_row[target_index] += recipe.product_rates[i];
                    }
                }
                
                matrix.push(new_row);
                cost_vec.push(recipe.base_power_consumption);
            }
        }
        
        let matrix_array = Array2::from_shape_vec(
            (matrix.len(), resources.len()),
            matrix.concat(),
        ).unwrap_or_else(|_| Array2::zeros((0, resources.len())));
        
        (matrix_array, cost_vec)
    }

    /// Sort resources topologically
    pub fn sort_topologically(&self, resource_list: &mut Vec<String>) {
        resource_list.sort_by_key(|v| {
            self.topological_sort_result
                .get(v)
                .map(|(_, finish)| u64::MAX - finish)
                .unwrap_or(0)
        });
    }

    /// Create a subgraph with selected recipes
    pub fn select(&self, selected_recipes: &[String]) -> Self {
        let mut new_graph = Self::new();
        
        for recipe_name in selected_recipes {
            if let Some(recipe) = self.recipes.get(recipe_name) {
                new_graph.add_recipe(recipe);
            }
        }
        
        new_graph.topological_sort();
        new_graph
    }

    /// Detect strongly connected components (cycles) in the graph using Tarjan's algorithm
    pub fn detect_cycles(&self) -> Vec<CycleInfo> {
        let mut cycles = Vec::new();
        let mut visited = HashMap::new();
        let mut stack = Vec::new();
        let mut on_stack = HashMap::new();
        let mut index_counter = 0;
        let mut indices = HashMap::new();
        let mut lowlinks = HashMap::new();

        for resource_name in self.resources.keys() {
            if !visited.contains_key(resource_name) {
                self.tarjan_dfs(
                    resource_name,
                    &mut visited,
                    &mut stack,
                    &mut on_stack,
                    &mut index_counter,
                    &mut indices,
                    &mut lowlinks,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// Tarjan's strongly connected components algorithm helper
    fn tarjan_dfs(
        &self,
        resource: &str,
        visited: &mut HashMap<String, bool>,
        stack: &mut Vec<String>,
        on_stack: &mut HashMap<String, bool>,
        index_counter: &mut usize,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        cycles: &mut Vec<CycleInfo>,
    ) {
        indices.insert(resource.to_string(), *index_counter);
        lowlinks.insert(resource.to_string(), *index_counter);
        *index_counter += 1;
        stack.push(resource.to_string());
        on_stack.insert(resource.to_string(), true);
        visited.insert(resource.to_string(), true);

        if let Some(resource_node) = self.resources.get(resource) {
            for edge in &resource_node.egress_edges {
                let successor = &edge.to;
                
                if !visited.contains_key(successor) {
                    self.tarjan_dfs(
                        successor,
                        visited,
                        stack,
                        on_stack,
                        index_counter,
                        indices,
                        lowlinks,
                        cycles,
                    );
                    let successor_lowlink = lowlinks[successor];
                    let current_lowlink = lowlinks[resource];
                    lowlinks.insert(resource.to_string(), current_lowlink.min(successor_lowlink));
                } else if *on_stack.get(successor).unwrap_or(&false) {
                    let successor_index = indices[successor];
                    let current_lowlink = lowlinks[resource];
                    lowlinks.insert(resource.to_string(), current_lowlink.min(successor_index));
                }
            }
        }

        // If resource is a root node, pop the stack and create SCC
        if lowlinks[resource] == indices[resource] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.insert(w.clone(), false);
                component.push(w.clone());
                if w == resource {
                    break;
                }
            }

            // Only consider it a cycle if it has more than one node or self-loops
            if component.len() > 1 || self.has_self_loop(resource) {
                let cycle_recipes = self.get_recipes_in_cycle(&component);
                cycles.push(CycleInfo {
                    resources: component,
                    recipes: cycle_recipes,
                    can_optimize_as_super_node: true, // We'll determine this later
                });
            }
        }
    }

    /// Check if a resource has a self-loop
    fn has_self_loop(&self, resource: &str) -> bool {
        if let Some(resource_node) = self.resources.get(resource) {
            resource_node.egress_edges.iter().any(|edge| edge.to == resource)
        } else {
            false
        }
    }

    /// Get all recipes involved in a cycle
    fn get_recipes_in_cycle(&self, cycle_resources: &[String]) -> Vec<String> {
        let mut cycle_recipes = Vec::new();
        let cycle_set: std::collections::HashSet<_> = cycle_resources.iter().collect();

        for resource in cycle_resources {
            if let Some(resource_node) = self.resources.get(resource) {
                for edge in &resource_node.egress_edges {
                    if cycle_set.contains(&edge.to) && !cycle_recipes.contains(&edge.recipe_name) {
                        cycle_recipes.push(edge.recipe_name.clone());
                    }
                }
            }
        }

        cycle_recipes
    }

    /// Enhanced dependency resolution that handles cycles
    pub fn resolve_dependencies_with_cycles(&self, targets: &[String]) -> DependencyResolution {
        let cycles = self.detect_cycles();
        let (resources, recipes) = self.find_all_related(targets);
        
        let has_cycles = !cycles.is_empty();
        DependencyResolution {
            resources,
            recipes,
            cycles,
            has_cycles,
        }
    }

    /// Enhanced matrix construction that handles circular dependencies
    pub fn construct_matrix_with_cycles(
        &self,
        recipes: &[String],
        resources: &[String],
    ) -> (Array2<f64>, Vec<f64>, Vec<CycleInfo>) {
        let cycles = self.detect_cycles();
        let (matrix, cost_vec) = self.construct_matrix(recipes, resources);
        
        // The matrix construction already handles cycles naturally through the constraint formulation
        // Each recipe contributes its inputs and outputs regardless of cycles
        (matrix, cost_vec, cycles)
    }
}

// Conversion functions between original and WASM structures
impl From<crate::graph::Recipe> for WasmRecipe {
    fn from(recipe: crate::graph::Recipe) -> Self {
        Self {
            recipe_name: recipe.recipe_name,
            display_name: None,
            resources: recipe.resources,
            resources_rates: recipe.resources_rates,
            products: recipe.products,
            product_rates: recipe.product_rates,
            base_power_consumption: recipe.power_consumption,
            production_method: recipe.production_method,
            unlock_tags: recipe.unlock_tags,
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        }
    }
}

impl From<WasmRecipe> for crate::graph::Recipe {
    fn from(wasm_recipe: WasmRecipe) -> Self {
        Self {
            recipe_name: wasm_recipe.recipe_name,
            resources: wasm_recipe.resources,
            resources_rates: wasm_recipe.resources_rates,
            products: wasm_recipe.products,
            product_rates: wasm_recipe.product_rates,
            power_consumption: wasm_recipe.base_power_consumption,
            production_method: wasm_recipe.production_method,
            unlock_tags: wasm_recipe.unlock_tags,
        }
    }
}