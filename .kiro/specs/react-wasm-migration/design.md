# Design Document

## Overview

This design document outlines the architecture for migrating the existing Satisfactory Production Chain Optimizer from a Yew-based frontend to a modern React + TypeScript frontend with an optimized WASM core. The system will maintain the existing mathematical foundation using incident matrices while adding advanced discrete optimization capabilities, interactive graph visualization, and enhanced user experience with flexible data management.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    React Frontend (TypeScript)              │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Input Panel   │  │  Visualization  │  │  Settings Panel │ │
│  │                 │  │                 │  │                 │ │
│  │ • Item Search   │  │ • Sankey Graph  │  │ • Recipe Presets│ │
│  │ • Target Rates  │  │ • Box-Line View │  │ • Custom Config │ │
│  │ • Data Upload   │  │ • Interactive   │  │ • Machine Select│ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    WASM Interface Layer                     │
├─────────────────────────────────────────────────────────────┤
│                    Rust WASM Core Engine                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  Graph Engine   │  │  Optimization   │  │  Data Manager   │ │
│  │                 │  │                 │  │                 │ │
│  │ • Incident      │  │ • Linear Prog   │  │ • Recipe Data   │ │
│  │   Matrix        │  │ • Discrete Opt  │  │ • Preset Config │ │
│  │ • Dependency    │  │ • CSP Solver    │  │ • Serialization │ │
│  │   Resolution    │  │ • MIP Solver    │  │                 │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│              User-Uploadable Data (JSON Files)              │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │   Game Data     │  │  Recipe Presets │                   │
│  │                 │  │                 │                   │
│  │ • Items/Recipes │  │ • Tier I Tech   │                   │
│  │ • Machines      │  │ • Tier II Tech  │                   │
│  │ • Default Data  │  │ • Custom Presets│                   │
│  └─────────────────┘  └─────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

**Frontend:**
- React 18+ with TypeScript for type safety and modern React features
- React Flow for interactive graph visualization
- Material-UI (MUI) or Tailwind CSS for consistent design system
- Vite for fast development and optimized builds
- React Query for state management and caching

**WASM Core:**
- Rust with wasm-bindgen for seamless JS interop
- serde for JSON serialization/deserialization
- ndarray for matrix operations (existing)
- New optimization libraries: HiGHS, OR-Tools bindings, or custom implementations

**Build & Deployment:**
- Pre-compiled WASM modules served as static assets
- Code splitting and lazy loading for optimization
- Static hosting compatible (Vercel, Netlify, GitHub Pages)

## Components and Interfaces

### Frontend Components

#### 1. Main Application Container
```typescript
interface AppState {
  productionTargets: ProductionTarget[];
  gameSettings: GameSettings;
  gameData: GameData | null;
  recipePresets: RecipePreset[];
  selectedPreset: string | null;
  customRecipeSelection: Set<string>;
  machineConfigurations: Map<string, MachineConfig>;
  calculationResults: CalculationResults | null;
  visualizationMode: 'sankey' | 'boxline';
  isCalculating: boolean;
}

interface ProductionTarget {
  itemName: string;
  rate: number;
  unit: 'minute' | 'hour' | 'second';
}

interface RecipePreset {
  id: string;
  name: string;
  description: string;
  enabledRecipes: Set<string>;
  machineConfigurations: Map<string, MachineConfig>;
}

interface MachineConfig {
  machineType: string;
  clockSpeed: number;
  efficiency: number;
  powerMultiplier: number;
}
```

#### 2. Data Upload Component
```typescript
interface DataUploadProps {
  onGameDataUpload: (data: GameData) => void;
  onPresetUpload: (presets: RecipePreset[]) => void;
  currentGameData: GameData | null;
  currentPresets: RecipePreset[];
}
```

#### 3. Recipe Configuration Component
```typescript
interface RecipeConfigProps {
  availableRecipes: Recipe[];
  selectedPreset: string | null;
  customSelection: Set<string>;
  onPresetSelect: (presetId: string) => void;
  onRecipeToggle: (recipeId: string, enabled: boolean) => void;
  onMachineConfigChange: (recipeId: string, config: MachineConfig) => void;
}
```

#### 4. Production Visualization Component
```typescript
interface ProductionVisualizationProps {
  results: CalculationResults;
  mode: 'sankey' | 'boxline';
  onRecipeToggle: (recipeId: string, enabled: boolean) => void;
  onNodeClick: (nodeId: string) => void;
  onMachineConfigChange: (recipeId: string, config: MachineConfig) => void;
}
```

### WASM Interface Layer

#### Core WASM Bindings
```rust
#[wasm_bindgen]
pub struct ProductionCalculator {
    graph: Graph,
    game_data: GameData,
    settings: CalculationSettings,
}

#[wasm_bindgen]
impl ProductionCalculator {
    #[wasm_bindgen(constructor)]
    pub fn new(game_data_json: &str) -> ProductionCalculator;
    
    #[wasm_bindgen]
    pub fn update_game_data(&mut self, game_data_json: &str) -> Result<(), JsValue>;
    
    #[wasm_bindgen]
    pub fn calculate_production(&self, targets: &JsValue) -> JsValue;
    
    #[wasm_bindgen]
    pub fn update_settings(&mut self, settings: &JsValue);
    
    #[wasm_bindgen]
    pub fn get_available_items(&self) -> JsValue;
    
    #[wasm_bindgen]
    pub fn validate_targets(&self, targets: &JsValue) -> JsValue;
    
    #[wasm_bindgen]
    pub fn apply_recipe_preset(&mut self, preset: &JsValue);
}
```

### Rust Core Engine Components

#### 1. Enhanced Graph Engine
```rust
pub struct Graph {
    pub recipes: HashMap<String, Recipe>,
    pub resources: HashMap<String, Rc<RefCell<ResourceNode>>>,
    pub machine_configs: HashMap<String, MachineConfig>,
    pub dependency_resolution_cache: HashMap<String, Vec<String>>,
}

impl Graph {
    // Existing methods preserved
    pub fn from_str(file: &str) -> Self;
    pub fn add_recipe(&mut self, recipe: &Recipe);
    pub fn find_all_related<T>(&self, targets: T) -> (Vec<String>, Vec<String>);
    
    // Enhanced methods for circular dependencies
    pub fn resolve_dependencies_with_cycles(&self, targets: &[String]) -> DependencyGraph;
    pub fn build_optimization_problem(&self, targets: &[ProductionTarget]) -> OptimizationProblem;
    pub fn calculate_flow_rates(&self, solution: &[f64]) -> HashMap<String, f64>;
    
    // Machine configuration support
    pub fn add_machine_config(&mut self, recipe_id: &str, config: MachineConfig);
    pub fn get_effective_recipe(&self, recipe_id: &str) -> Recipe;
}

// Dependency resolution without requiring topological sort
pub struct DependencyGraph {
    pub nodes: HashMap<String, DependencyNode>,
    pub cycles: Vec<CycleInfo>,
}

pub struct CycleInfo {
    pub recipes: Vec<String>,
    pub resources: Vec<String>,
    pub can_optimize_as_super_node: bool,
}
```

#### 2. Discrete Optimization Engine
```rust
pub trait OptimizationSolver {
    fn solve(&self, problem: &OptimizationProblem) -> OptimizationResult;
    fn validate_solution(&self, solution: &[f64]) -> bool;
    fn supports_cycles(&self) -> bool;
}

pub struct MixedIntegerSolver {
    // HiGHS or similar MIP solver integration
    // Handles cycles naturally through constraint formulation
}

pub struct ConstraintSatisfactionSolver {
    // CSP solver for recipe selection and feasibility
    // Can handle circular dependencies through constraint propagation
}

pub struct HybridOptimizer {
    linear_solver: LinearSolver,
    discrete_solver: MixedIntegerSolver,
    csp_solver: ConstraintSatisfactionSolver,
}

impl HybridOptimizer {
    pub fn solve_with_cycle_optimization(&self, problem: &OptimizationProblem) -> OptimizationResult {
        // Detect cycles and create super-nodes where beneficial
        // Use appropriate solver based on problem characteristics
    }
}
```

#### 3. Enhanced Data Management Layer
```rust
pub struct GameData {
    pub items: HashMap<String, Item>,
    pub recipes: HashMap<String, Recipe>,
    pub machines: HashMap<String, Machine>,
    pub recipe_presets: HashMap<String, RecipePreset>,
}

impl GameData {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>;
    pub fn merge_with_user_data(&mut self, user_data: &str) -> Result<(), serde_json::Error>;
    pub fn apply_preset(&self, preset_id: &str) -> Result<HashSet<String>, String>;
    pub fn validate_recipe_compatibility(&self, recipes: &[String]) -> Vec<String>;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecipePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled_recipes: HashSet<String>,
    pub machine_configurations: HashMap<String, MachineConfig>,
    pub technology_requirements: Vec<String>,
}
```

## Data Models

### Enhanced Recipe Model
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Recipe {
    pub recipe_name: String,
    pub display_name: String,
    pub resources: Vec<String>,
    pub resources_rates: Vec<f64>,
    pub products: Vec<String>,
    pub product_rates: Vec<f64>,
    pub base_power_consumption: f64,
    pub production_method: Vec<String>,
    pub unlock_tags: Vec<String>,
    pub is_alternate: bool,
    pub default_machine_type: String,
    pub production_time: f64,
    pub allows_machine_selection: bool,
    pub compatible_machines: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MachineConfig {
    pub machine_type: String,
    pub clock_speed: f64, // 0.01 to 2.5 (1% to 250%)
    pub efficiency: f64,
    pub power_consumption_multiplier: f64,
}
```

### Machine Model
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Machine {
    pub name: String,
    pub display_name: String,
    pub base_power_consumption: f64,
    pub power_consumption_exponent: f64,
    pub manufacturing_speed: f64,
    pub compatible_recipes: Vec<String>,
    pub max_clock_speed: f64,
    pub min_clock_speed: f64,
}
```

## Dependency Resolution Strategy

### Handling Circular Dependencies

Instead of relying solely on topological sort (which fails with cycles), the system will use:

#### 1. Cycle Detection and Analysis
```rust
impl Graph {
    pub fn analyze_cycles(&self) -> Vec<CycleAnalysis> {
        // Use Tarjan's strongly connected components algorithm
        // Analyze each cycle for optimization opportunities
    }
    
    pub fn can_create_super_node(&self, cycle: &[String]) -> bool {
        // Determine if a cycle can be optimized as a single super-recipe
        // Based on resource flow balance and optimization constraints
    }
}

pub struct CycleAnalysis {
    pub recipes: Vec<String>,
    pub resources: Vec<String>,
    pub is_self_contained: bool,
    pub optimization_benefit: f64,
}
```

#### 2. Constraint-Based Resolution
```rust
pub struct DependencyResolver {
    pub fn resolve_with_constraints(&self, targets: &[ProductionTarget]) -> OptimizationProblem {
        // Create constraint system that naturally handles cycles
        // No need for topological ordering
        // Let optimization solver handle the dependency relationships
    }
}
```

#### 3. Super-Node Optimization
```rust
pub struct SuperNode {
    pub contained_recipes: Vec<String>,
    pub net_inputs: HashMap<String, f64>,
    pub net_outputs: HashMap<String, f64>,
    pub internal_power_consumption: f64,
}

impl SuperNode {
    pub fn from_cycle(cycle: &CycleAnalysis, graph: &Graph) -> Option<Self> {
        // Create optimized super-node representation of circular dependencies
        // Only if it provides computational or user experience benefits
    }
}
```

## Error Handling

### Enhanced Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum CalculationError {
    #[error("Invalid production target: {0}")]
    InvalidTarget(String),
    
    #[error("Incompatible recipe selection: {0}")]
    IncompatibleRecipes(String),
    
    #[error("Machine configuration error: {0}")]
    MachineConfigError(String),
    
    #[error("No feasible solution found")]
    NoFeasibleSolution,
    
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    
    #[error("Data parsing error: {0}")]
    DataParsingError(#[from] serde_json::Error),
    
    #[error("Preset not found: {0}")]
    PresetNotFound(String),
}
```

## Testing Strategy

### Enhanced Testing for New Features

#### Cycle Handling Tests
```rust
#[cfg(test)]
mod cycle_tests {
    #[test]
    fn test_aluminum_plutonium_cycle_resolution() {
        // Test complex circular dependencies
    }
    
    #[test]
    fn test_super_node_optimization() {
        // Verify super-node creation and optimization
    }
    
    #[test]
    fn test_multiple_machine_configurations() {
        // Test different machine setups for same recipe
    }
}
```

#### Data Management Tests
```rust
#[cfg(test)]
mod data_tests {
    #[test]
    fn test_user_data_upload_and_merge() {
        // Test custom game data integration
    }
    
    #[test]
    fn test_preset_application() {
        // Test recipe preset functionality
    }
}
```

## Performance Considerations

### Cycle Optimization
- Cache cycle analysis results for repeated calculations
- Use super-nodes only when they provide measurable performance benefits
- Implement lazy evaluation for complex dependency chains

### Data Management
- Implement efficient diff algorithms for game data updates
- Use incremental updates for recipe configuration changes
- Cache optimization problem formulations for similar configurations

## Security and Validation

### User Data Validation
- Validate uploaded JSON structure and content
- Sanitize recipe names and prevent code injection
- Verify recipe mathematical consistency (conservation of mass/energy)

### Preset Validation
- Ensure preset recipes are compatible with current game data
- Validate machine configurations are within acceptable ranges
- Check for circular preset dependencies