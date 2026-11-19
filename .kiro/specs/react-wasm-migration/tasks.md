# Implementation Plan

## Overview

This implementation plan converts the React + WASM migration design into a series of discrete, manageable coding tasks. Each task builds incrementally on previous work, ensuring early testing and validation of core functionality. The plan prioritizes establishing the WASM foundation first, then building the React frontend, and finally adding advanced optimization features.

## Implementation Tasks

- [x] 1. Set up project structure and build system
  - Create new React + TypeScript project with Vite
  - Configure wasm-pack build integration
  - Set up development and production build scripts
  - Create basic project directory structure for frontend and WASM modules
  - _Requirements: 1.1, 1.3_

- [x] 2. Migrate core Rust graph engine to WASM module
  - [x] 2.1 Create WASM-compatible graph data structures
    - Port existing Graph, Recipe, and ResourceNode structures to WASM module
    - Add wasm-bindgen annotations for JavaScript interop
    - Implement serialization/deserialization for data transfer
    - _Requirements: 1.1, 1.4_

  - [x] 2.2 Implement WASM bindings for graph operations
    - Create ProductionCalculator WASM struct with constructor
    - Implement calculate_production method with JsValue input/output
    - Add methods for game data loading and validation
    - Write unit tests for WASM bindings
    - _Requirements: 1.1, 1.5_

  - [x] 2.3 Port existing matrix construction and dependency resolution
    - Migrate find_all_related method to WASM module
    - Port construct_matrix functionality with enhanced cycle handling
    - Implement dependency resolution without topological sort requirement
    - Create tests for circular dependency scenarios (aluminum/plutonium)
    - _Requirements: 1.4, 4.1, 4.2_

- [x] 3. Implement enhanced data management system
  - [x] 3.1 Create flexible game data loading system
    - Implement GameData struct with JSON parsing
    - Add support for merging user-uploaded data with default data
    - Create validation functions for recipe mathematical consistency
    - Write tests for data loading and validation
    - _Requirements: 4.1, 9.1, 9.6_

  - [x] 3.2 Implement recipe preset system
    - Create RecipePreset data structure with serialization
    - Implement preset loading from JSON files
    - Add preset application and validation logic
    - Create default presets for different technology tiers
    - _Requirements: 9.2, 9.5_

  - [x] 3.3 Add machine configuration support
    - Implement MachineConfig struct with clock speed and efficiency settings
    - Add machine compatibility checking for recipes
    - Create methods for applying machine configurations to recipes
    - Write tests for machine configuration validation
    - _Requirements: 9.3, 9.4_

- [ ] 4. Implement discrete optimization engine
  - [ ] 4.1 Create optimization problem abstraction
    - Define OptimizationProblem struct with constraints and variables
    - Implement ObjectiveFunction enum for different optimization goals
    - Create constraint generation from production targets and recipe selection
    - _Requirements: 6.1, 6.2_

  - [ ] 4.2 Implement Mixed Integer Programming solver
    - Integrate HiGHS or similar MIP solver library
    - Create MixedIntegerSolver implementing OptimizationSolver trait
    - Implement integer constraints for machine counts
    - Add support for discrete clock speed optimization
    - _Requirements: 6.3, 6.4, 6.5_

  - [ ] 4.3 Add constraint satisfaction solver for recipe selection
    - Implement ConstraintSatisfactionSolver for feasibility checking
    - Add recipe compatibility and dependency constraint handling
    - Create efficient recipe selection algorithms
    - Write tests for complex recipe selection scenarios
    - _Requirements: 6.2, 6.6_

- [ ] 5. Create React frontend foundation
  - [ ] 5.1 Set up main application structure
    - Create main App component with TypeScript interfaces
    - Implement AppState management with React hooks
    - Set up routing and basic layout components
    - Add error boundary for robust error handling
    - _Requirements: 2.1, 2.2, 11.1_

  - [ ] 5.2 Implement WASM integration layer
    - Create React hooks for WASM module loading and initialization
    - Implement async WASM method calling with proper error handling
    - Add loading states and progress indicators for calculations
    - Create TypeScript type definitions for WASM interfaces
    - _Requirements: 2.1, 11.2_

  - [ ] 5.3 Create production target input interface
    - Implement ItemSelector component with fuzzy search functionality
    - Create production target list management (add/remove/edit)
    - Add input validation and real-time feedback
    - Implement keyboard navigation and accessibility features
    - _Requirements: 8.1, 8.2, 8.3, 8.7_

- [ ] 6. Implement data upload and configuration interface
  - [ ] 6.1 Create game data upload component
    - Implement file upload interface for JSON game data
    - Add drag-and-drop functionality for data files
    - Create data validation and error reporting UI
    - Implement data merging preview before application
    - _Requirements: 9.1, 9.6, 10.1_

  - [ ] 6.2 Implement recipe preset management
    - Create preset selection dropdown with descriptions
    - Implement individual recipe checkbox interface
    - Add preset upload and custom preset creation
    - Create preset export functionality
    - _Requirements: 9.2, 9.5, 10.4_

  - [ ] 6.3 Add machine configuration interface
    - Create machine selection UI for compatible recipes
    - Implement clock speed and efficiency input controls
    - Add real-time power consumption calculation display
    - Create machine configuration presets and templates
    - _Requirements: 9.3, 9.4_

- [ ] 7. Implement production visualization system
  - [ ] 7.1 Set up React Flow graph visualization
    - Install and configure React Flow library
    - Create basic node and edge components for production chains
    - Implement graph layout algorithms for clear visualization
    - Add zoom, pan, and selection functionality
    - _Requirements: 3.1, 3.4, 11.3_

  - [ ] 7.2 Create Sankey diagram visualization mode
    - Implement Sankey diagram rendering using D3.js or similar
    - Add flow rate visualization with proportional edge widths
    - Create interactive hover and click functionality
    - Implement smooth transitions between different configurations
    - _Requirements: 3.1, 3.2_

  - [ ] 7.3 Add interactive graph editing capabilities
    - Implement recipe enable/disable directly from graph nodes
    - Add machine configuration editing from node context menus
    - Create real-time recalculation on graph modifications
    - Add visual feedback for configuration changes
    - _Requirements: 3.3, 3.5, 3.6_

- [ ] 8. Implement calculation results and export system
  - [ ] 8.1 Create results display components
    - Implement detailed table view for recipe requirements
    - Create summary view with key metrics (power, resources, machines)
    - Add filtering and sorting capabilities for large result sets
    - Implement expandable sections for detailed recipe information
    - _Requirements: 8.4, 8.6, 10.2_

  - [ ] 8.2 Add export and sharing functionality
    - Implement CSV export for material lists and shopping lists
    - Create URL encoding for complete application state
    - Add JSON export for configuration backup
    - Implement shareable link generation with compression
    - _Requirements: 10.1, 10.3, 10.4_

  - [ ] 8.3 Create comparison and optimization metrics display
    - Implement side-by-side comparison of different optimization results
    - Add Pareto frontier visualization for multi-objective optimization
    - Create efficiency metrics and resource utilization displays
    - Implement optimization recommendation system
    - _Requirements: 8.5, 6.6_

- [ ] 9. Add advanced optimization features
  - [ ] 9.1 Implement cycle detection and super-node optimization
    - Create Tarjan's algorithm implementation for strongly connected components
    - Add cycle analysis and optimization benefit calculation
    - Implement super-node creation for beneficial cycles
    - Write comprehensive tests for complex circular dependencies
    - _Requirements: 4.1, 4.2, 6.2_

  - [ ] 9.2 Add multi-objective optimization
    - Implement weighted objective function combinations
    - Create Pareto optimization for conflicting objectives
    - Add user interface for objective weight configuration
    - Implement optimization result ranking and selection
    - _Requirements: 5.1, 5.2, 5.5, 6.6_

  - [ ] 9.3 Implement factory partitioning system
    - Create graph partitioning algorithms (Fiduccia-Mattheyses)
    - Add transportation cost calculation between partitions
    - Implement partition visualization and editing
    - Create partition export for factory planning
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 10. Performance optimization and polish
  - [ ] 10.1 Optimize WASM performance
    - Implement object pooling for frequently allocated structures
    - Add caching for repeated matrix calculations
    - Optimize memory usage and implement proper cleanup
    - Add performance benchmarks and monitoring
    - _Requirements: 11.1, 11.5_

  - [ ] 10.2 Implement frontend performance optimizations
    - Add virtual scrolling for large item lists
    - Implement React.memo and useMemo for expensive calculations
    - Add debouncing for real-time input updates
    - Optimize bundle size with code splitting
    - _Requirements: 11.3, 11.4, 11.7_

  - [ ] 10.3 Add responsive design and mobile support
    - Implement responsive layouts for tablet and mobile devices
    - Add touch-friendly controls for graph interaction
    - Create mobile-optimized input interfaces
    - Test and optimize performance on mobile devices
    - _Requirements: 8.7, 11.6_

- [ ] 11. Testing and deployment preparation
  - [ ] 11.1 Create comprehensive test suite
    - Write unit tests for all WASM functions and React components
    - Add integration tests for complete calculation workflows
    - Create end-to-end tests for user scenarios
    - Implement performance regression tests
    - _Requirements: All requirements validation_

  - [ ] 11.2 Prepare production build and deployment
    - Optimize WASM binary size and loading performance
    - Configure static asset caching and compression
    - Set up CI/CD pipeline for automated testing and deployment
    - Create deployment documentation and user guides
    - _Requirements: 11.7, 1.2_

  - [ ] 11.3 Add error handling and user feedback
    - Implement comprehensive error boundaries and recovery
    - Add user-friendly error messages and help text
    - Create tutorial and onboarding flow for new users
    - Add analytics and error reporting for production monitoring
    - _Requirements: 8.8, 11.2_