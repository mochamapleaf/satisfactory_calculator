use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::graph::Recipe;

/// Enhanced Recipe model with additional fields for machine configuration and metadata
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct EnhancedRecipe {
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

/// Machine configuration with clock speed and efficiency settings
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct MachineConfig {
    pub machine_type: String,
    pub clock_speed: f64, // 0.01 to 2.5 (1% to 250%)
    pub efficiency: f64,
    pub power_consumption_multiplier: f64,
}

/// Machine model with power consumption and compatibility information
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Machine {
    pub name: String,
    pub display_name: Option<String>,
    pub base_power_consumption: f64,
    pub power_consumption_exponent: f64,
    pub manufacturing_speed: f64,
    pub compatible_recipes: Vec<String>,
    pub max_clock_speed: f64,
    pub min_clock_speed: f64,
}

/// Item model for game items
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub stack_size: Option<u32>,
    pub is_fluid: Option<bool>,
}

/// Recipe preset for different technology tiers or configurations
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct RecipePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled_recipes: HashSet<String>,
    pub machine_configurations: HashMap<String, MachineConfig>,
    pub technology_requirements: Vec<String>,
}

/// Main game data structure that holds all game information
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GameData {
    pub items: HashMap<String, Item>,
    pub recipes: HashMap<String, EnhancedRecipe>,
    pub machines: HashMap<String, Machine>,
    pub recipe_presets: HashMap<String, RecipePreset>,
}

/// Validation error types for game data
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Recipe '{0}' has mismatched resource and rate counts")]
    MismatchedResourceRates(String),
    
    #[error("Recipe '{0}' has mismatched product and rate counts")]
    MismatchedProductRates(String),
    
    #[error("Recipe '{0}' references unknown item '{1}'")]
    UnknownItem(String, String),
    
    #[error("Recipe '{0}' references unknown machine '{1}'")]
    UnknownMachine(String, String),
    
    #[error("Machine config for recipe '{0}' has invalid clock speed: {1}")]
    InvalidClockSpeed(String, f64),
    
    #[error("Recipe '{0}' has negative rates")]
    NegativeRates(String),
    
    #[error("Recipe '{0}' has zero production time")]
    ZeroProductionTime(String),
    
    #[error("Preset '{0}' references unknown recipe '{1}'")]
    PresetUnknownRecipe(String, String),
}

impl Default for GameData {
    fn default() -> Self {
        Self {
            items: HashMap::new(),
            recipes: HashMap::new(),
            machines: HashMap::new(),
            recipe_presets: HashMap::new(),
        }
    }
}

impl GameData {
    /// Create GameData from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    /// Create GameData from legacy recipe format (Vec<Recipe>)
    pub fn from_legacy_recipes(recipes: Vec<Recipe>) -> Self {
        let mut game_data = GameData::default();
        
        for recipe in recipes {
            // Convert legacy Recipe to EnhancedRecipe
            let enhanced_recipe = EnhancedRecipe {
                recipe_name: recipe.recipe_name.clone(),
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
            };
            
            game_data.recipes.insert(recipe.recipe_name, enhanced_recipe);
        }
        
        game_data
    }
    
    /// Merge user-uploaded data with existing game data
    pub fn merge_with_user_data(&mut self, user_data_json: &str) -> Result<(), serde_json::Error> {
        let user_data: GameData = serde_json::from_str(user_data_json)?;
        
        // Merge items
        for (name, item) in user_data.items {
            self.items.insert(name, item);
        }
        
        // Merge recipes
        for (name, recipe) in user_data.recipes {
            self.recipes.insert(name, recipe);
        }
        
        // Merge machines
        for (name, machine) in user_data.machines {
            self.machines.insert(name, machine);
        }
        
        // Merge presets
        for (id, preset) in user_data.recipe_presets {
            self.recipe_presets.insert(id, preset);
        }
        
        Ok(())
    }
    
    /// Apply a recipe preset and return the set of enabled recipes
    pub fn apply_preset(&self, preset_id: &str) -> Result<HashSet<String>, String> {
        let preset = self.recipe_presets.get(preset_id)
            .ok_or_else(|| format!("Preset '{}' not found", preset_id))?;
        
        // Validate that all recipes in the preset exist
        for recipe_name in &preset.enabled_recipes {
            if !self.recipes.contains_key(recipe_name) {
                return Err(format!("Preset '{}' references unknown recipe '{}'", preset_id, recipe_name));
            }
        }
        
        Ok(preset.enabled_recipes.clone())
    }
    
    /// Validate recipe compatibility with available recipes and machines
    pub fn validate_recipe_compatibility(&self, recipe_names: &[String]) -> Vec<String> {
        let mut errors = Vec::new();
        
        for recipe_name in recipe_names {
            if !self.recipes.contains_key(recipe_name) {
                errors.push(format!("Recipe '{}' not found", recipe_name));
            }
        }
        
        errors
    }
    
    /// Validate the mathematical consistency of all recipes
    pub fn validate_recipes(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        for (recipe_name, recipe) in &self.recipes {
            // Check resource and rate count consistency
            if recipe.resources.len() != recipe.resources_rates.len() {
                errors.push(ValidationError::MismatchedResourceRates(recipe_name.clone()));
            }
            
            if recipe.products.len() != recipe.product_rates.len() {
                errors.push(ValidationError::MismatchedProductRates(recipe_name.clone()));
            }
            
            // Check for negative rates
            if recipe.resources_rates.iter().any(|&rate| rate < 0.0) || 
               recipe.product_rates.iter().any(|&rate| rate < 0.0) {
                errors.push(ValidationError::NegativeRates(recipe_name.clone()));
            }
            
            // Check production time
            if let Some(production_time) = recipe.production_time {
                if production_time <= 0.0 {
                    errors.push(ValidationError::ZeroProductionTime(recipe_name.clone()));
                }
            }
            
            // Validate items exist (if items are defined)
            if !self.items.is_empty() {
                for item_name in recipe.resources.iter().chain(recipe.products.iter()) {
                    if !self.items.contains_key(item_name) {
                        errors.push(ValidationError::UnknownItem(recipe_name.clone(), item_name.clone()));
                    }
                }
            }
            
            // Validate machines exist (if machines are defined)
            if !self.machines.is_empty() {
                if let Some(ref compatible_machines) = recipe.compatible_machines {
                    for machine_name in compatible_machines {
                        if !self.machines.contains_key(machine_name) {
                            errors.push(ValidationError::UnknownMachine(recipe_name.clone(), machine_name.clone()));
                        }
                    }
                }
                
                if let Some(ref default_machine) = recipe.default_machine_type {
                    if !self.machines.contains_key(default_machine) {
                        errors.push(ValidationError::UnknownMachine(recipe_name.clone(), default_machine.clone()));
                    }
                }
            }
        }
        
        // Validate presets
        for (preset_id, preset) in &self.recipe_presets {
            for recipe_name in &preset.enabled_recipes {
                if !self.recipes.contains_key(recipe_name) {
                    errors.push(ValidationError::PresetUnknownRecipe(preset_id.clone(), recipe_name.clone()));
                }
            }
            
            // Validate machine configurations in presets
            for (recipe_name, machine_config) in &preset.machine_configurations {
                if machine_config.clock_speed < 0.01 || machine_config.clock_speed > 2.5 {
                    errors.push(ValidationError::InvalidClockSpeed(recipe_name.clone(), machine_config.clock_speed));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Get all available items as a sorted vector
    pub fn get_available_items(&self) -> Vec<String> {
        let mut items: Vec<String> = self.items.keys().cloned().collect();
        items.sort();
        items
    }
    
    /// Get all available recipes as a sorted vector
    pub fn get_available_recipes(&self) -> Vec<String> {
        let mut recipes: Vec<String> = self.recipes.keys().cloned().collect();
        recipes.sort();
        recipes
    }
    
    /// Convert enhanced recipes back to legacy Recipe format for compatibility
    pub fn to_legacy_recipes(&self) -> Vec<Recipe> {
        self.recipes.values().map(|enhanced| Recipe {
            recipe_name: enhanced.recipe_name.clone(),
            resources: enhanced.resources.clone(),
            resources_rates: enhanced.resources_rates.clone(),
            products: enhanced.products.clone(),
            product_rates: enhanced.product_rates.clone(),
            power_consumption: enhanced.base_power_consumption,
            production_method: enhanced.production_method.clone(),
            unlock_tags: enhanced.unlock_tags.clone(),
        }).collect()
    }
}

impl MachineConfig {
    /// Create a default machine configuration
    pub fn default_config(machine_type: String) -> Self {
        Self {
            machine_type,
            clock_speed: 1.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        }
    }
    
    /// Validate machine configuration parameters
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.clock_speed < 0.01 || self.clock_speed > 2.5 {
            return Err(ValidationError::InvalidClockSpeed(
                self.machine_type.clone(), 
                self.clock_speed
            ));
        }
        Ok(())
    }
    
    /// Calculate effective power consumption based on clock speed
    pub fn calculate_power_consumption(&self, base_power: f64) -> f64 {
        base_power * self.power_consumption_multiplier * self.clock_speed.powf(1.6)
    }
    
    /// Calculate effective production rate based on clock speed and efficiency
    pub fn calculate_production_rate(&self, base_rate: f64) -> f64 {
        base_rate * self.clock_speed * self.efficiency
    }
}

impl Machine {
    /// Check if this machine is compatible with a given recipe
    pub fn is_compatible_with_recipe(&self, recipe_name: &str) -> bool {
        self.compatible_recipes.contains(&recipe_name.to_string())
    }
    
    /// Validate clock speed is within machine limits
    pub fn validate_clock_speed(&self, clock_speed: f64) -> bool {
        clock_speed >= self.min_clock_speed && clock_speed <= self.max_clock_speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_game_data_creation() {
        let game_data = GameData::default();
        assert!(game_data.items.is_empty());
        assert!(game_data.recipes.is_empty());
        assert!(game_data.machines.is_empty());
        assert!(game_data.recipe_presets.is_empty());
    }
    
    #[test]
    fn test_machine_config_validation() {
        let valid_config = MachineConfig::default_config("Smelter".to_string());
        assert!(valid_config.validate().is_ok());
        
        let invalid_config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 3.0, // Invalid: > 2.5
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        assert!(invalid_config.validate().is_err());
    }
    
    #[test]
    fn test_power_consumption_calculation() {
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 2.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        let base_power = 4.0;
        let calculated_power = config.calculate_power_consumption(base_power);
        // Power should be base * clock_speed^1.6
        let expected_power = 4.0 * 2.0_f64.powf(1.6);
        assert!((calculated_power - expected_power).abs() < 0.001);
    }
    
    #[test]
    fn test_production_rate_calculation() {
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 1.5,
            efficiency: 0.9,
            power_consumption_multiplier: 1.0,
        };
        
        let base_rate = 30.0;
        let calculated_rate = config.calculate_production_rate(base_rate);
        let expected_rate = 30.0 * 1.5 * 0.9;
        assert!((calculated_rate - expected_rate).abs() < 0.001);
    }
}    #[
test]
    fn test_legacy_recipe_conversion() {
        let legacy_recipe = Recipe {
            recipe_name: "Iron Ingot".to_string(),
            resources: vec!["Iron Ore".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Ingot".to_string()],
            product_rates: vec![30.0],
            power_consumption: 4.0,
            production_method: vec!["Smelter".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
        };
        
        let game_data = GameData::from_legacy_recipes(vec![legacy_recipe.clone()]);
        assert_eq!(game_data.recipes.len(), 1);
        
        let enhanced_recipe = &game_data.recipes["Iron Ingot"];
        assert_eq!(enhanced_recipe.recipe_name, legacy_recipe.recipe_name);
        assert_eq!(enhanced_recipe.base_power_consumption, legacy_recipe.power_consumption);
        
        // Convert back to legacy format
        let converted_back = game_data.to_legacy_recipes();
        assert_eq!(converted_back.len(), 1);
        assert_eq!(converted_back[0], legacy_recipe);
    }
    
    #[test]
    fn test_json_serialization() {
        let mut game_data = GameData::default();
        
        // Add a test item
        let item = Item {
            name: "Iron Ore".to_string(),
            display_name: Some("Iron Ore".to_string()),
            category: Some("Raw Materials".to_string()),
            stack_size: Some(100),
            is_fluid: Some(false),
        };
        game_data.items.insert("Iron Ore".to_string(), item);
        
        // Add a test recipe
        let recipe = EnhancedRecipe {
            recipe_name: "Iron Ingot".to_string(),
            display_name: Some("Iron Ingot".to_string()),
            resources: vec!["Iron Ore".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Ingot".to_string()],
            product_rates: vec![30.0],
            base_power_consumption: 4.0,
            production_method: vec!["Smelter".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: Some(false),
            default_machine_type: Some("Smelter".to_string()),
            production_time: Some(2.0),
            allows_machine_selection: Some(false),
            compatible_machines: Some(vec!["Smelter".to_string()]),
        };
        game_data.recipes.insert("Iron Ingot".to_string(), recipe);
        
        // Serialize to JSON
        let json = serde_json::to_string(&game_data).expect("Failed to serialize");
        
        // Deserialize back
        let deserialized: GameData = GameData::from_json(&json).expect("Failed to deserialize");
        
        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.recipes.len(), 1);
        assert!(deserialized.items.contains_key("Iron Ore"));
        assert!(deserialized.recipes.contains_key("Iron Ingot"));
    }
    
    #[test]
    fn test_data_merging() {
        let mut base_data = GameData::default();
        
        // Add base item
        let base_item = Item {
            name: "Iron Ore".to_string(),
            display_name: Some("Iron Ore".to_string()),
            category: Some("Raw Materials".to_string()),
            stack_size: Some(100),
            is_fluid: Some(false),
        };
        base_data.items.insert("Iron Ore".to_string(), base_item);
        
        // Create user data JSON
        let user_data_json = r#"{
            "items": {
                "Copper Ore": {
                    "name": "Copper Ore",
                    "display_name": "Copper Ore",
                    "category": "Raw Materials",
                    "stack_size": 100,
                    "is_fluid": false
                }
            },
            "recipes": {
                "Copper Ingot": {
                    "recipe_name": "Copper Ingot",
                    "display_name": "Copper Ingot",
                    "resources": ["Copper Ore"],
                    "resources_rates": [30.0],
                    "products": ["Copper Ingot"],
                    "product_rates": [30.0],
                    "base_power_consumption": 4.0,
                    "production_method": ["Smelter"],
                    "unlock_tags": ["Tier 0"]
                }
            },
            "machines": {},
            "recipe_presets": {}
        }"#;
        
        // Merge user data
        base_data.merge_with_user_data(user_data_json).expect("Failed to merge data");
        
        // Verify merge results
        assert_eq!(base_data.items.len(), 2);
        assert!(base_data.items.contains_key("Iron Ore"));
        assert!(base_data.items.contains_key("Copper Ore"));
        assert_eq!(base_data.recipes.len(), 1);
        assert!(base_data.recipes.contains_key("Copper Ingot"));
    }
    
    #[test]
    fn test_recipe_validation() {
        let mut game_data = GameData::default();
        
        // Add valid recipe
        let valid_recipe = EnhancedRecipe {
            recipe_name: "Valid Recipe".to_string(),
            display_name: None,
            resources: vec!["Resource A".to_string()],
            resources_rates: vec![10.0],
            products: vec!["Product A".to_string()],
            product_rates: vec![5.0],
            base_power_consumption: 2.0,
            production_method: vec!["Machine".to_string()],
            unlock_tags: vec!["Tier 1".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: Some(1.0),
            allows_machine_selection: None,
            compatible_machines: None,
        };
        game_data.recipes.insert("Valid Recipe".to_string(), valid_recipe);
        
        // Add invalid recipe with mismatched rates
        let invalid_recipe = EnhancedRecipe {
            recipe_name: "Invalid Recipe".to_string(),
            display_name: None,
            resources: vec!["Resource B".to_string(), "Resource C".to_string()],
            resources_rates: vec![10.0], // Mismatch: 2 resources, 1 rate
            products: vec!["Product B".to_string()],
            product_rates: vec![5.0, 3.0], // Mismatch: 1 product, 2 rates
            base_power_consumption: 2.0,
            production_method: vec!["Machine".to_string()],
            unlock_tags: vec!["Tier 1".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: Some(0.0), // Invalid: zero production time
            allows_machine_selection: None,
            compatible_machines: None,
        };
        game_data.recipes.insert("Invalid Recipe".to_string(), invalid_recipe);
        
        // Validate recipes
        let validation_result = game_data.validate_recipes();
        assert!(validation_result.is_err());
        
        let errors = validation_result.unwrap_err();
        assert!(errors.len() >= 3); // Should have at least 3 errors
        
        // Check for specific error types
        let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert!(error_messages.iter().any(|msg| msg.contains("mismatched resource")));
        assert!(error_messages.iter().any(|msg| msg.contains("mismatched product")));
        assert!(error_messages.iter().any(|msg| msg.contains("zero production time")));
    }
    
    #[test]
    fn test_preset_functionality() {
        let mut game_data = GameData::default();
        
        // Add recipes
        let recipe1 = EnhancedRecipe {
            recipe_name: "Recipe 1".to_string(),
            display_name: None,
            resources: vec!["A".to_string()],
            resources_rates: vec![1.0],
            products: vec!["B".to_string()],
            product_rates: vec![1.0],
            base_power_consumption: 1.0,
            production_method: vec!["Machine".to_string()],
            unlock_tags: vec!["Tier 1".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        let recipe2 = EnhancedRecipe {
            recipe_name: "Recipe 2".to_string(),
            display_name: None,
            resources: vec!["B".to_string()],
            resources_rates: vec![1.0],
            products: vec!["C".to_string()],
            product_rates: vec![1.0],
            base_power_consumption: 2.0,
            production_method: vec!["Machine".to_string()],
            unlock_tags: vec!["Tier 2".to_string()],
            is_alternate: None,
            default_machine_type: None,
            production_time: None,
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        game_data.recipes.insert("Recipe 1".to_string(), recipe1);
        game_data.recipes.insert("Recipe 2".to_string(), recipe2);
        
        // Create preset
        let mut enabled_recipes = HashSet::new();
        enabled_recipes.insert("Recipe 1".to_string());
        enabled_recipes.insert("Recipe 2".to_string());
        
        let preset = RecipePreset {
            id: "tier1_preset".to_string(),
            name: "Tier 1 Preset".to_string(),
            description: "Basic tier 1 recipes".to_string(),
            enabled_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Tier 1".to_string()],
        };
        
        game_data.recipe_presets.insert("tier1_preset".to_string(), preset);
        
        // Test preset application
        let applied_recipes = game_data.apply_preset("tier1_preset").expect("Failed to apply preset");
        assert_eq!(applied_recipes.len(), 2);
        assert!(applied_recipes.contains("Recipe 1"));
        assert!(applied_recipes.contains("Recipe 2"));
        
        // Test invalid preset
        let invalid_result = game_data.apply_preset("nonexistent_preset");
        assert!(invalid_result.is_err());
    }
    
    #[test]
    fn test_machine_compatibility() {
        let machine = Machine {
            name: "Smelter".to_string(),
            display_name: Some("Smelter".to_string()),
            base_power_consumption: 4.0,
            power_consumption_exponent: 1.6,
            manufacturing_speed: 1.0,
            compatible_recipes: vec!["Iron Ingot".to_string(), "Copper Ingot".to_string()],
            max_clock_speed: 2.5,
            min_clock_speed: 0.01,
        };
        
        assert!(machine.is_compatible_with_recipe("Iron Ingot"));
        assert!(machine.is_compatible_with_recipe("Copper Ingot"));
        assert!(!machine.is_compatible_with_recipe("Steel Ingot"));
        
        assert!(machine.validate_clock_speed(1.0));
        assert!(machine.validate_clock_speed(2.5));
        assert!(machine.validate_clock_speed(0.01));
        assert!(!machine.validate_clock_speed(3.0));
        assert!(!machine.validate_clock_speed(0.005));
    }
/// Preset manager for handling recipe presets and technology tiers
pub struct PresetManager {
    pub presets: HashMap<String, RecipePreset>,
}

impl PresetManager {
    /// Create a new preset manager with default presets
    pub fn new() -> Self {
        let mut manager = Self {
            presets: HashMap::new(),
        };
        manager.add_default_presets();
        manager
    }
    
    /// Add default presets for different technology tiers
    fn add_default_presets(&mut self) {
        // Tier 0 - Basic Resources
        let tier0_recipes = [
            "Iron Ingot", "Copper Ingot", "Limestone", "Coal",
        ].iter().map(|s| s.to_string()).collect();
        
        let tier0_preset = RecipePreset {
            id: "tier0_basic".to_string(),
            name: "Tier 0 - Basic Resources".to_string(),
            description: "Basic smelting and resource extraction recipes".to_string(),
            enabled_recipes: tier0_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Tier 0".to_string()],
        };
        self.presets.insert("tier0_basic".to_string(), tier0_preset);
        
        // Tier 1 - Basic Production
        let tier1_recipes = [
            "Iron Ingot", "Copper Ingot", "Limestone", "Coal",
            "Iron Plate", "Iron Rod", "Copper Sheet", "Wire",
            "Cable", "Concrete", "Screw", "Reinforced Iron Plate",
        ].iter().map(|s| s.to_string()).collect();
        
        let tier1_preset = RecipePreset {
            id: "tier1_basic_production".to_string(),
            name: "Tier 1 - Basic Production".to_string(),
            description: "Basic manufacturing recipes for early game".to_string(),
            enabled_recipes: tier1_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Tier 0".to_string(), "Tier 1".to_string()],
        };
        self.presets.insert("tier1_basic_production".to_string(), tier1_preset);
        
        // Tier 2 - Advanced Production
        let tier2_recipes = [
            "Iron Ingot", "Copper Ingot", "Limestone", "Coal",
            "Iron Plate", "Iron Rod", "Copper Sheet", "Wire", "Cable", "Concrete", "Screw",
            "Reinforced Iron Plate", "Rotor", "Modular Frame", "Smart Plating",
            "Steel Ingot", "Steel Beam", "Steel Pipe", "Versatile Framework",
        ].iter().map(|s| s.to_string()).collect();
        
        let tier2_preset = RecipePreset {
            id: "tier2_steel_production".to_string(),
            name: "Tier 2 - Steel Production".to_string(),
            description: "Steel production and advanced manufacturing".to_string(),
            enabled_recipes: tier2_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Tier 0".to_string(), "Tier 1".to_string(), "Tier 2".to_string()],
        };
        self.presets.insert("tier2_steel_production".to_string(), tier2_preset);
        
        // Tier 3 - Oil Processing
        let tier3_recipes = [
            "Iron Ingot", "Copper Ingot", "Limestone", "Coal", "Steel Ingot",
            "Iron Plate", "Iron Rod", "Copper Sheet", "Wire", "Cable", "Concrete", "Screw",
            "Reinforced Iron Plate", "Rotor", "Modular Frame", "Steel Beam", "Steel Pipe",
            "Crude Oil", "Plastic", "Rubber", "Fuel", "Heavy Oil Residue",
            "Circuit Board", "Computer", "Heavy Modular Frame", "Motor",
        ].iter().map(|s| s.to_string()).collect();
        
        let tier3_preset = RecipePreset {
            id: "tier3_oil_processing".to_string(),
            name: "Tier 3 - Oil Processing".to_string(),
            description: "Oil refining and plastic production".to_string(),
            enabled_recipes: tier3_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Tier 0".to_string(), "Tier 1".to_string(), "Tier 2".to_string(), "Tier 3".to_string()],
        };
        self.presets.insert("tier3_oil_processing".to_string(), tier3_preset);
        
        // Alternate Recipes Preset
        let alternate_recipes = [
            "Iron Wire", "Casted Screw", "Steel Screw", "Solid Steel Ingot",
            "Copper Rotor", "Steel Rotor", "Stitched Iron Plate", "Adhered Iron Plate",
            "Bolted Iron Plate", "Recycled Plastic", "Recycled Rubber",
        ].iter().map(|s| s.to_string()).collect();
        
        let alternate_preset = RecipePreset {
            id: "alternate_recipes".to_string(),
            name: "Alternate Recipes".to_string(),
            description: "Collection of alternate recipes for optimization".to_string(),
            enabled_recipes: alternate_recipes,
            machine_configurations: HashMap::new(),
            technology_requirements: vec!["Hard Drive Research".to_string()],
        };
        self.presets.insert("alternate_recipes".to_string(), alternate_preset);
    }
    
    /// Load presets from JSON file
    pub fn load_presets_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let loaded_presets: HashMap<String, RecipePreset> = serde_json::from_str(json)?;
        for (id, preset) in loaded_presets {
            self.presets.insert(id, preset);
        }
        Ok(())
    }
    
    /// Export presets to JSON
    pub fn export_presets_to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.presets)
    }
    
    /// Get a preset by ID
    pub fn get_preset(&self, id: &str) -> Option<&RecipePreset> {
        self.presets.get(id)
    }
    
    /// Add or update a preset
    pub fn add_preset(&mut self, preset: RecipePreset) {
        self.presets.insert(preset.id.clone(), preset);
    }
    
    /// Remove a preset
    pub fn remove_preset(&mut self, id: &str) -> Option<RecipePreset> {
        self.presets.remove(id)
    }
    
    /// Get all preset IDs sorted by name
    pub fn get_preset_ids(&self) -> Vec<String> {
        let mut presets: Vec<_> = self.presets.values().collect();
        presets.sort_by(|a, b| a.name.cmp(&b.name));
        presets.iter().map(|p| p.id.clone()).collect()
    }
    
    /// Validate a preset against available recipes
    pub fn validate_preset(&self, preset_id: &str, available_recipes: &HashSet<String>) -> Result<(), Vec<String>> {
        let preset = self.presets.get(preset_id)
            .ok_or_else(|| vec![format!("Preset '{}' not found", preset_id)])?;
        
        let mut errors = Vec::new();
        for recipe_name in &preset.enabled_recipes {
            if !available_recipes.contains(recipe_name) {
                errors.push(format!("Recipe '{}' not available", recipe_name));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Create a custom preset from a list of recipe names
    pub fn create_custom_preset(
        &mut self,
        id: String,
        name: String,
        description: String,
        recipe_names: Vec<String>,
        machine_configs: HashMap<String, MachineConfig>,
    ) -> RecipePreset {
        let preset = RecipePreset {
            id: id.clone(),
            name,
            description,
            enabled_recipes: recipe_names.into_iter().collect(),
            machine_configurations: machine_configs,
            technology_requirements: vec!["Custom".to_string()],
        };
        
        self.presets.insert(id, preset.clone());
        preset
    }
    
    /// Merge two presets into a new preset
    pub fn merge_presets(&mut self, preset1_id: &str, preset2_id: &str, new_id: String, new_name: String) -> Result<RecipePreset, String> {
        let preset1 = self.presets.get(preset1_id)
            .ok_or_else(|| format!("Preset '{}' not found", preset1_id))?;
        let preset2 = self.presets.get(preset2_id)
            .ok_or_else(|| format!("Preset '{}' not found", preset2_id))?;
        
        let mut merged_recipes = preset1.enabled_recipes.clone();
        merged_recipes.extend(preset2.enabled_recipes.iter().cloned());
        
        let mut merged_configs = preset1.machine_configurations.clone();
        merged_configs.extend(preset2.machine_configurations.iter().map(|(k, v)| (k.clone(), v.clone())));
        
        let mut merged_tech_requirements = preset1.technology_requirements.clone();
        merged_tech_requirements.extend(preset2.technology_requirements.iter().cloned());
        merged_tech_requirements.sort();
        merged_tech_requirements.dedup();
        
        let merged_preset = RecipePreset {
            id: new_id.clone(),
            name: new_name,
            description: format!("Merged from '{}' and '{}'", preset1.name, preset2.name),
            enabled_recipes: merged_recipes,
            machine_configurations: merged_configs,
            technology_requirements: merged_tech_requirements,
        };
        
        self.presets.insert(new_id, merged_preset.clone());
        Ok(merged_preset)
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData {
    /// Get the preset manager with default presets
    pub fn get_preset_manager(&self) -> PresetManager {
        let mut manager = PresetManager::new();
        
        // Add any presets from the game data
        for (id, preset) in &self.recipe_presets {
            manager.presets.insert(id.clone(), preset.clone());
        }
        
        manager
    }
    
    /// Apply preset and get machine configurations
    pub fn apply_preset_with_configs(&self, preset_id: &str) -> Result<(HashSet<String>, HashMap<String, MachineConfig>), String> {
        let preset = self.recipe_presets.get(preset_id)
            .ok_or_else(|| format!("Preset '{}' not found", preset_id))?;
        
        // Validate that all recipes in the preset exist
        for recipe_name in &preset.enabled_recipes {
            if !self.recipes.contains_key(recipe_name) {
                return Err(format!("Preset '{}' references unknown recipe '{}'", preset_id, recipe_name));
            }
        }
        
        Ok((preset.enabled_recipes.clone(), preset.machine_configurations.clone()))
    }
    
    /// Get recipes filtered by technology tier
    pub fn get_recipes_by_tier(&self, tier: &str) -> Vec<String> {
        self.recipes.values()
            .filter(|recipe| recipe.unlock_tags.contains(&tier.to_string()))
            .map(|recipe| recipe.recipe_name.clone())
            .collect()
    }
    
    /// Get all available technology tiers
    pub fn get_available_tiers(&self) -> Vec<String> {
        let mut tiers: HashSet<String> = HashSet::new();
        for recipe in self.recipes.values() {
            for tag in &recipe.unlock_tags {
                tiers.insert(tag.clone());
            }
        }
        let mut tier_vec: Vec<String> = tiers.into_iter().collect();
        tier_vec.sort();
        tier_vec
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;
    
    #[test]
    fn test_preset_manager_creation() {
        let manager = PresetManager::new();
        assert!(!manager.presets.is_empty());
        
        // Check that default presets exist
        assert!(manager.get_preset("tier0_basic").is_some());
        assert!(manager.get_preset("tier1_basic_production").is_some());
        assert!(manager.get_preset("tier2_steel_production").is_some());
        assert!(manager.get_preset("tier3_oil_processing").is_some());
        assert!(manager.get_preset("alternate_recipes").is_some());
    }
    
    #[test]
    fn test_preset_validation() {
        let manager = PresetManager::new();
        let mut available_recipes = HashSet::new();
        available_recipes.insert("Iron Ingot".to_string());
        available_recipes.insert("Copper Ingot".to_string());
        available_recipes.insert("Limestone".to_string());
        available_recipes.insert("Coal".to_string());
        
        // Should pass validation for tier0 preset
        let result = manager.validate_preset("tier0_basic", &available_recipes);
        assert!(result.is_ok());
        
        // Should fail validation for tier1 preset (missing recipes)
        let result = manager.validate_preset("tier1_basic_production", &available_recipes);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_custom_preset_creation() {
        let mut manager = PresetManager::new();
        let initial_count = manager.presets.len();
        
        let recipe_names = vec!["Custom Recipe 1".to_string(), "Custom Recipe 2".to_string()];
        let machine_configs = HashMap::new();
        
        let custom_preset = manager.create_custom_preset(
            "custom_test".to_string(),
            "Test Custom Preset".to_string(),
            "A test preset".to_string(),
            recipe_names.clone(),
            machine_configs,
        );
        
        assert_eq!(manager.presets.len(), initial_count + 1);
        assert_eq!(custom_preset.enabled_recipes.len(), 2);
        assert!(custom_preset.enabled_recipes.contains("Custom Recipe 1"));
        assert!(custom_preset.enabled_recipes.contains("Custom Recipe 2"));
    }
    
    #[test]
    fn test_preset_merging() {
        let mut manager = PresetManager::new();
        
        let result = manager.merge_presets(
            "tier0_basic",
            "tier1_basic_production",
            "merged_test".to_string(),
            "Merged Test Preset".to_string(),
        );
        
        assert!(result.is_ok());
        let merged_preset = result.unwrap();
        
        // Should contain recipes from both presets
        let tier0_preset = manager.get_preset("tier0_basic").unwrap();
        let tier1_preset = manager.get_preset("tier1_basic_production").unwrap();
        
        for recipe in &tier0_preset.enabled_recipes {
            assert!(merged_preset.enabled_recipes.contains(recipe));
        }
        for recipe in &tier1_preset.enabled_recipes {
            assert!(merged_preset.enabled_recipes.contains(recipe));
        }
    }
    
    #[test]
    fn test_preset_json_serialization() {
        let manager = PresetManager::new();
        
        // Export to JSON
        let json = manager.export_presets_to_json().expect("Failed to export presets");
        assert!(!json.is_empty());
        
        // Create new manager and load presets
        let mut new_manager = PresetManager { presets: HashMap::new() };
        new_manager.load_presets_from_json(&json).expect("Failed to load presets");
        
        // Should have the same number of presets
        assert_eq!(new_manager.presets.len(), manager.presets.len());
        
        // Check specific preset
        let original_preset = manager.get_preset("tier0_basic").unwrap();
        let loaded_preset = new_manager.get_preset("tier0_basic").unwrap();
        assert_eq!(original_preset.name, loaded_preset.name);
        assert_eq!(original_preset.enabled_recipes, loaded_preset.enabled_recipes);
    }
    
    #[test]
    fn test_game_data_preset_integration() {
        let mut game_data = GameData::default();
        
        // Add some test recipes
        let recipe1 = EnhancedRecipe {
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
        game_data.recipes.insert("Iron Ingot".to_string(), recipe1);
        
        // Get preset manager
        let manager = game_data.get_preset_manager();
        assert!(!manager.presets.is_empty());
        
        // Test tier filtering
        let tier0_recipes = game_data.get_recipes_by_tier("Tier 0");
        assert_eq!(tier0_recipes.len(), 1);
        assert!(tier0_recipes.contains(&"Iron Ingot".to_string()));
        
        // Test available tiers
        let tiers = game_data.get_available_tiers();
        assert!(tiers.contains(&"Tier 0".to_string()));
    }
}

/// Machine configuration manager for handling machine setups and recipe compatibility
pub struct MachineConfigManager {
    pub machines: HashMap<String, Machine>,
    pub configurations: HashMap<String, MachineConfig>, // recipe_name -> config
}

impl MachineConfigManager {
    /// Create a new machine configuration manager
    pub fn new(machines: HashMap<String, Machine>) -> Self {
        Self {
            machines,
            configurations: HashMap::new(),
        }
    }
    
    /// Add a machine configuration for a specific recipe
    pub fn add_configuration(&mut self, recipe_name: String, config: MachineConfig) -> Result<(), String> {
        // Validate the configuration
        config.validate().map_err(|e| e.to_string())?;
        
        // Check if the machine exists
        if !self.machines.contains_key(&config.machine_type) {
            return Err(format!("Machine '{}' not found", config.machine_type));
        }
        
        // Validate clock speed against machine limits
        let machine = &self.machines[&config.machine_type];
        if !machine.validate_clock_speed(config.clock_speed) {
            return Err(format!(
                "Clock speed {:.2} is outside machine limits ({:.2} - {:.2})",
                config.clock_speed, machine.min_clock_speed, machine.max_clock_speed
            ));
        }
        
        self.configurations.insert(recipe_name, config);
        Ok(())
    }
    
    /// Get machine configuration for a recipe
    pub fn get_configuration(&self, recipe_name: &str) -> Option<&MachineConfig> {
        self.configurations.get(recipe_name)
    }
    
    /// Remove machine configuration for a recipe
    pub fn remove_configuration(&mut self, recipe_name: &str) -> Option<MachineConfig> {
        self.configurations.remove(recipe_name)
    }
    
    /// Check if a machine is compatible with a recipe
    pub fn is_machine_compatible(&self, machine_name: &str, recipe_name: &str) -> bool {
        if let Some(machine) = self.machines.get(machine_name) {
            machine.is_compatible_with_recipe(recipe_name)
        } else {
            false
        }
    }
    
    /// Get all compatible machines for a recipe
    pub fn get_compatible_machines(&self, recipe_name: &str) -> Vec<String> {
        self.machines.values()
            .filter(|machine| machine.is_compatible_with_recipe(recipe_name))
            .map(|machine| machine.name.clone())
            .collect()
    }
    
    /// Apply machine configuration to a recipe and get effective rates
    pub fn apply_configuration_to_recipe(&self, recipe: &EnhancedRecipe) -> EnhancedRecipe {
        let mut modified_recipe = recipe.clone();
        
        if let Some(config) = self.configurations.get(&recipe.recipe_name) {
            // Apply clock speed and efficiency to production rates
            for rate in &mut modified_recipe.resources_rates {
                *rate = config.calculate_production_rate(*rate);
            }
            for rate in &mut modified_recipe.product_rates {
                *rate = config.calculate_production_rate(*rate);
            }
            
            // Apply power consumption changes
            modified_recipe.base_power_consumption = config.calculate_power_consumption(recipe.base_power_consumption);
            
            // Adjust production time if present
            if let Some(production_time) = modified_recipe.production_time {
                modified_recipe.production_time = Some(production_time / config.clock_speed);
            }
        }
        
        modified_recipe
    }
    
    /// Get total power consumption for all configured recipes
    pub fn calculate_total_power_consumption(&self, recipes: &HashMap<String, EnhancedRecipe>) -> f64 {
        let mut total_power = 0.0;
        
        for (recipe_name, recipe) in recipes {
            if let Some(config) = self.configurations.get(recipe_name) {
                total_power += config.calculate_power_consumption(recipe.base_power_consumption);
            } else {
                total_power += recipe.base_power_consumption;
            }
        }
        
        total_power
    }
    
    /// Validate all machine configurations
    pub fn validate_all_configurations(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        for (recipe_name, config) in &self.configurations {
            if let Err(e) = config.validate() {
                errors.push(format!("Recipe '{}': {}", recipe_name, e));
            }
            
            if let Some(machine) = self.machines.get(&config.machine_type) {
                if !machine.validate_clock_speed(config.clock_speed) {
                    errors.push(format!(
                        "Recipe '{}': Clock speed {:.2} outside machine limits ({:.2} - {:.2})",
                        recipe_name, config.clock_speed, machine.min_clock_speed, machine.max_clock_speed
                    ));
                }
            } else {
                errors.push(format!("Recipe '{}': Machine '{}' not found", recipe_name, config.machine_type));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Create default configurations for all recipes using their default machines
    pub fn create_default_configurations(&mut self, recipes: &HashMap<String, EnhancedRecipe>) {
        for (recipe_name, recipe) in recipes {
            if let Some(ref default_machine) = recipe.default_machine_type {
                if self.machines.contains_key(default_machine) {
                    let config = MachineConfig::default_config(default_machine.clone());
                    self.configurations.insert(recipe_name.clone(), config);
                }
            }
        }
    }
    
    /// Export configurations to JSON
    pub fn export_configurations(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.configurations)
    }
    
    /// Import configurations from JSON
    pub fn import_configurations(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let imported_configs: HashMap<String, MachineConfig> = serde_json::from_str(json)?;
        self.configurations.extend(imported_configs);
        Ok(())
    }
    
    /// Get machine efficiency statistics
    pub fn get_efficiency_stats(&self) -> MachineEfficiencyStats {
        let mut stats = MachineEfficiencyStats::default();
        
        for config in self.configurations.values() {
            stats.total_machines += 1;
            stats.total_clock_speed += config.clock_speed;
            stats.total_efficiency += config.efficiency;
            
            if config.clock_speed > 1.0 {
                stats.overclocked_machines += 1;
            } else if config.clock_speed < 1.0 {
                stats.underclocked_machines += 1;
            }
        }
        
        if stats.total_machines > 0 {
            stats.average_clock_speed = stats.total_clock_speed / stats.total_machines as f64;
            stats.average_efficiency = stats.total_efficiency / stats.total_machines as f64;
        }
        
        stats
    }
}

/// Statistics for machine efficiency analysis
#[derive(Debug, Default, Clone)]
pub struct MachineEfficiencyStats {
    pub total_machines: u32,
    pub overclocked_machines: u32,
    pub underclocked_machines: u32,
    pub average_clock_speed: f64,
    pub average_efficiency: f64,
    pub total_clock_speed: f64,
    pub total_efficiency: f64,
}

impl GameData {
    /// Get machine configuration manager
    pub fn get_machine_config_manager(&self) -> MachineConfigManager {
        MachineConfigManager::new(self.machines.clone())
    }
    
    /// Apply machine configurations to all recipes
    pub fn apply_machine_configurations(&self, config_manager: &MachineConfigManager) -> HashMap<String, EnhancedRecipe> {
        self.recipes.iter()
            .map(|(name, recipe)| (name.clone(), config_manager.apply_configuration_to_recipe(recipe)))
            .collect()
    }
    
    /// Get default machine for a recipe
    pub fn get_default_machine_for_recipe(&self, recipe_name: &str) -> Option<String> {
        self.recipes.get(recipe_name)
            .and_then(|recipe| recipe.default_machine_type.clone())
    }
    
    /// Get all machines that can produce a specific item
    pub fn get_machines_for_item(&self, item_name: &str) -> Vec<String> {
        let mut machines = Vec::new();
        
        // Find recipes that produce this item
        let producing_recipes: Vec<&EnhancedRecipe> = self.recipes.values()
            .filter(|recipe| recipe.products.contains(&item_name.to_string()))
            .collect();
        
        // Get machines for those recipes
        for recipe in producing_recipes {
            if let Some(ref compatible_machines) = recipe.compatible_machines {
                machines.extend(compatible_machines.clone());
            } else if let Some(ref default_machine) = recipe.default_machine_type {
                machines.push(default_machine.clone());
            }
        }
        
        machines.sort();
        machines.dedup();
        machines
    }
    
    /// Validate machine configurations against recipes
    pub fn validate_machine_configurations(&self, configurations: &HashMap<String, MachineConfig>) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        for (recipe_name, config) in configurations {
            // Check if recipe exists
            if let Some(recipe) = self.recipes.get(recipe_name) {
                // Check if machine is compatible
                if let Some(ref compatible_machines) = recipe.compatible_machines {
                    if !compatible_machines.contains(&config.machine_type) {
                        errors.push(format!(
                            "Machine '{}' is not compatible with recipe '{}'",
                            config.machine_type, recipe_name
                        ));
                    }
                }
                
                // Check if machine exists
                if !self.machines.contains_key(&config.machine_type) {
                    errors.push(format!("Machine '{}' not found", config.machine_type));
                }
            } else {
                errors.push(format!("Recipe '{}' not found", recipe_name));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Helper function to create default machines for testing
pub fn create_default_machines() -> HashMap<String, Machine> {
    let mut machines = HashMap::new();
    
    // Smelter
    machines.insert("Smelter".to_string(), Machine {
        name: "Smelter".to_string(),
        display_name: Some("Smelter".to_string()),
        base_power_consumption: 4.0,
        power_consumption_exponent: 1.6,
        manufacturing_speed: 1.0,
        compatible_recipes: vec![
            "Iron Ingot".to_string(),
            "Copper Ingot".to_string(),
            "Steel Ingot".to_string(),
        ],
        max_clock_speed: 2.5,
        min_clock_speed: 0.01,
    });
    
    // Constructor
    machines.insert("Constructor".to_string(), Machine {
        name: "Constructor".to_string(),
        display_name: Some("Constructor".to_string()),
        base_power_consumption: 4.0,
        power_consumption_exponent: 1.6,
        manufacturing_speed: 1.0,
        compatible_recipes: vec![
            "Iron Plate".to_string(),
            "Iron Rod".to_string(),
            "Copper Sheet".to_string(),
            "Wire".to_string(),
            "Cable".to_string(),
            "Concrete".to_string(),
            "Screw".to_string(),
        ],
        max_clock_speed: 2.5,
        min_clock_speed: 0.01,
    });
    
    // Assembler
    machines.insert("Assembler".to_string(), Machine {
        name: "Assembler".to_string(),
        display_name: Some("Assembler".to_string()),
        base_power_consumption: 15.0,
        power_consumption_exponent: 1.6,
        manufacturing_speed: 1.0,
        compatible_recipes: vec![
            "Reinforced Iron Plate".to_string(),
            "Rotor".to_string(),
            "Modular Frame".to_string(),
            "Smart Plating".to_string(),
        ],
        max_clock_speed: 2.5,
        min_clock_speed: 0.01,
    });
    
    // Foundry
    machines.insert("Foundry".to_string(), Machine {
        name: "Foundry".to_string(),
        display_name: Some("Foundry".to_string()),
        base_power_consumption: 16.0,
        power_consumption_exponent: 1.6,
        manufacturing_speed: 1.0,
        compatible_recipes: vec![
            "Steel Ingot".to_string(),
            "Steel Beam".to_string(),
            "Steel Pipe".to_string(),
        ],
        max_clock_speed: 2.5,
        min_clock_speed: 0.01,
    });
    
    machines
}

#[cfg(test)]
mod machine_config_tests {
    use super::*;
    
    #[test]
    fn test_machine_config_manager_creation() {
        let machines = create_default_machines();
        let manager = MachineConfigManager::new(machines);
        
        assert_eq!(manager.machines.len(), 4);
        assert!(manager.configurations.is_empty());
    }
    
    #[test]
    fn test_add_valid_configuration() {
        let machines = create_default_machines();
        let mut manager = MachineConfigManager::new(machines);
        
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 1.5,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        let result = manager.add_configuration("Iron Ingot".to_string(), config.clone());
        assert!(result.is_ok());
        
        let retrieved_config = manager.get_configuration("Iron Ingot");
        assert!(retrieved_config.is_some());
        assert_eq!(retrieved_config.unwrap().clock_speed, 1.5);
    }
    
    #[test]
    fn test_add_invalid_configuration() {
        let machines = create_default_machines();
        let mut manager = MachineConfigManager::new(machines);
        
        // Invalid clock speed
        let invalid_config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 3.0, // Too high
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        let result = manager.add_configuration("Iron Ingot".to_string(), invalid_config);
        assert!(result.is_err());
        
        // Non-existent machine
        let nonexistent_machine_config = MachineConfig {
            machine_type: "NonExistentMachine".to_string(),
            clock_speed: 1.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        let result = manager.add_configuration("Iron Ingot".to_string(), nonexistent_machine_config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_machine_compatibility() {
        let machines = create_default_machines();
        let manager = MachineConfigManager::new(machines);
        
        assert!(manager.is_machine_compatible("Smelter", "Iron Ingot"));
        assert!(manager.is_machine_compatible("Constructor", "Iron Plate"));
        assert!(!manager.is_machine_compatible("Smelter", "Iron Plate"));
        assert!(!manager.is_machine_compatible("NonExistent", "Iron Ingot"));
    }
    
    #[test]
    fn test_get_compatible_machines() {
        let machines = create_default_machines();
        let manager = MachineConfigManager::new(machines);
        
        let compatible = manager.get_compatible_machines("Iron Ingot");
        assert!(compatible.contains(&"Smelter".to_string()));
        assert!(!compatible.contains(&"Constructor".to_string()));
        
        let compatible = manager.get_compatible_machines("Iron Plate");
        assert!(compatible.contains(&"Constructor".to_string()));
        assert!(!compatible.contains(&"Smelter".to_string()));
    }
    
    #[test]
    fn test_apply_configuration_to_recipe() {
        let machines = create_default_machines();
        let mut manager = MachineConfigManager::new(machines);
        
        let recipe = EnhancedRecipe {
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
            default_machine_type: Some("Smelter".to_string()),
            production_time: Some(2.0),
            allows_machine_selection: None,
            compatible_machines: None,
        };
        
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 2.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        manager.add_configuration("Iron Ingot".to_string(), config).unwrap();
        
        let modified_recipe = manager.apply_configuration_to_recipe(&recipe);
        
        // Rates should be doubled due to 2.0 clock speed
        assert_eq!(modified_recipe.resources_rates[0], 60.0);
        assert_eq!(modified_recipe.product_rates[0], 60.0);
        
        // Power consumption should increase (4.0 * 2.0^1.6)
        let expected_power = 4.0 * 2.0_f64.powf(1.6);
        assert!((modified_recipe.base_power_consumption - expected_power).abs() < 0.001);
        
        // Production time should be halved
        assert_eq!(modified_recipe.production_time, Some(1.0));
    }
    
    #[test]
    fn test_efficiency_stats() {
        let machines = create_default_machines();
        let mut manager = MachineConfigManager::new(machines);
        
        // Add various configurations
        manager.add_configuration("Recipe1".to_string(), MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 2.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        }).unwrap();
        
        manager.add_configuration("Recipe2".to_string(), MachineConfig {
            machine_type: "Constructor".to_string(),
            clock_speed: 0.5,
            efficiency: 0.9,
            power_consumption_multiplier: 1.0,
        }).unwrap();
        
        manager.add_configuration("Recipe3".to_string(), MachineConfig {
            machine_type: "Assembler".to_string(),
            clock_speed: 1.0,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        }).unwrap();
        
        let stats = manager.get_efficiency_stats();
        
        assert_eq!(stats.total_machines, 3);
        assert_eq!(stats.overclocked_machines, 1);
        assert_eq!(stats.underclocked_machines, 1);
        assert_eq!(stats.average_clock_speed, (2.0 + 0.5 + 1.0) / 3.0);
        assert_eq!(stats.average_efficiency, (1.0 + 0.9 + 1.0) / 3.0);
    }
    
    #[test]
    fn test_configuration_serialization() {
        let machines = create_default_machines();
        let mut manager = MachineConfigManager::new(machines);
        
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 1.5,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        manager.add_configuration("Iron Ingot".to_string(), config).unwrap();
        
        // Export configurations
        let json = manager.export_configurations().expect("Failed to export");
        assert!(!json.is_empty());
        
        // Create new manager and import
        let machines2 = create_default_machines();
        let mut manager2 = MachineConfigManager::new(machines2);
        manager2.import_configurations(&json).expect("Failed to import");
        
        let imported_config = manager2.get_configuration("Iron Ingot");
        assert!(imported_config.is_some());
        assert_eq!(imported_config.unwrap().clock_speed, 1.5);
    }
}#[
cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_complete_data_management_workflow() {
        // Create a complete game data setup
        let mut game_data = GameData::default();
        
        // Add machines
        game_data.machines = create_default_machines();
        
        // Add items
        let iron_ore = Item {
            name: "Iron Ore".to_string(),
            display_name: Some("Iron Ore".to_string()),
            category: Some("Raw Materials".to_string()),
            stack_size: Some(100),
            is_fluid: Some(false),
        };
        let iron_ingot = Item {
            name: "Iron Ingot".to_string(),
            display_name: Some("Iron Ingot".to_string()),
            category: Some("Ingots".to_string()),
            stack_size: Some(100),
            is_fluid: Some(false),
        };
        game_data.items.insert("Iron Ore".to_string(), iron_ore);
        game_data.items.insert("Iron Ingot".to_string(), iron_ingot);
        
        // Add recipes
        let iron_ingot_recipe = EnhancedRecipe {
            recipe_name: "Iron Ingot".to_string(),
            display_name: Some("Iron Ingot".to_string()),
            resources: vec!["Iron Ore".to_string()],
            resources_rates: vec![30.0],
            products: vec!["Iron Ingot".to_string()],
            product_rates: vec![30.0],
            base_power_consumption: 4.0,
            production_method: vec!["Smelter".to_string()],
            unlock_tags: vec!["Tier 0".to_string()],
            is_alternate: Some(false),
            default_machine_type: Some("Smelter".to_string()),
            production_time: Some(2.0),
            allows_machine_selection: Some(false),
            compatible_machines: Some(vec!["Smelter".to_string()]),
        };
        game_data.recipes.insert("Iron Ingot".to_string(), iron_ingot_recipe);
        
        // Test data validation
        let validation_result = game_data.validate_recipes();
        assert!(validation_result.is_ok(), "Game data should be valid");
        
        // Test preset system
        let preset_manager = game_data.get_preset_manager();
        let tier0_recipes = preset_manager.apply_preset("tier0_basic").unwrap();
        assert!(tier0_recipes.contains("Iron Ingot"));
        
        // Test machine configuration
        let mut machine_manager = game_data.get_machine_config_manager();
        
        // Add a machine configuration
        let config = MachineConfig {
            machine_type: "Smelter".to_string(),
            clock_speed: 1.5,
            efficiency: 1.0,
            power_consumption_multiplier: 1.0,
        };
        
        machine_manager.add_configuration("Iron Ingot".to_string(), config).unwrap();
        
        // Apply configurations to recipes
        let configured_recipes = game_data.apply_machine_configurations(&machine_manager);
        let configured_recipe = &configured_recipes["Iron Ingot"];
        
        // Verify the configuration was applied
        assert_eq!(configured_recipe.resources_rates[0], 45.0); // 30.0 * 1.5
        assert_eq!(configured_recipe.product_rates[0], 45.0); // 30.0 * 1.5
        
        // Test power consumption calculation
        let expected_power = 4.0 * 1.5_f64.powf(1.6);
        assert!((configured_recipe.base_power_consumption - expected_power).abs() < 0.001);
        
        // Test serialization of the complete system
        let game_data_json = serde_json::to_string(&game_data).unwrap();
        let deserialized_game_data: GameData = GameData::from_json(&game_data_json).unwrap();
        
        assert_eq!(deserialized_game_data.recipes.len(), game_data.recipes.len());
        assert_eq!(deserialized_game_data.machines.len(), game_data.machines.len());
        assert_eq!(deserialized_game_data.items.len(), game_data.items.len());
        
        println!("✅ Complete data management workflow test passed!");
    }
    
    #[test]
    fn test_user_data_upload_workflow() {
        // Start with base game data
        let mut base_game_data = GameData::default();
        base_game_data.machines = create_default_machines();
        
        // Simulate user uploading custom data
        let user_data_json = r#"{
            "items": {
                "Custom Ore": {
                    "name": "Custom Ore",
                    "display_name": "Custom Ore",
                    "category": "Raw Materials",
                    "stack_size": 200,
                    "is_fluid": false
                }
            },
            "recipes": {
                "Custom Ingot": {
                    "recipe_name": "Custom Ingot",
                    "display_name": "Custom Ingot",
                    "resources": ["Custom Ore"],
                    "resources_rates": [20.0],
                    "products": ["Custom Ingot"],
                    "product_rates": [10.0],
                    "base_power_consumption": 8.0,
                    "production_method": ["Smelter"],
                    "unlock_tags": ["Custom Tier"],
                    "is_alternate": false,
                    "default_machine_type": "Smelter",
                    "production_time": 4.0,
                    "compatible_machines": ["Smelter"]
                }
            },
            "machines": {},
            "recipe_presets": {
                "custom_preset": {
                    "id": "custom_preset",
                    "name": "Custom Preset",
                    "description": "User's custom recipe preset",
                    "enabled_recipes": ["Custom Ingot"],
                    "machine_configurations": {
                        "Custom Ingot": {
                            "machine_type": "Smelter",
                            "clock_speed": 2.0,
                            "efficiency": 1.0,
                            "power_consumption_multiplier": 1.0
                        }
                    },
                    "technology_requirements": ["Custom Tier"]
                }
            }
        }"#;
        
        // Merge user data
        base_game_data.merge_with_user_data(user_data_json).unwrap();
        
        // Validate the merged data
        let validation_result = base_game_data.validate_recipes();
        assert!(validation_result.is_ok());
        
        // Test that user data was properly integrated
        assert!(base_game_data.items.contains_key("Custom Ore"));
        assert!(base_game_data.recipes.contains_key("Custom Ingot"));
        assert!(base_game_data.recipe_presets.contains_key("custom_preset"));
        
        // Test applying the custom preset
        let (enabled_recipes, machine_configs) = base_game_data.apply_preset_with_configs("custom_preset").unwrap();
        assert!(enabled_recipes.contains("Custom Ingot"));
        assert!(machine_configs.contains_key("Custom Ingot"));
        
        let custom_config = &machine_configs["Custom Ingot"];
        assert_eq!(custom_config.clock_speed, 2.0);
        
        println!("✅ User data upload workflow test passed!");
    }
}