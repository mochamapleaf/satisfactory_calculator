use serde::{Deserialize, Serialize};
//use std::fs::File;
//use std::io::prelude::*;
use ndarray::Array2;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hasher;
//Hash
use std::rc::Rc;
//use minilp::{Problem, OptimizationDirection, ComparisonOp};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Recipe {
    pub recipe_name: String,
    pub resources: Vec<String>,
    pub resources_rates: Vec<f64>,
    pub products: Vec<String>,
    pub product_rates: Vec<f64>,
    pub power_consumption: f64,
    pub production_method: Vec<String>,
    pub unlock_tags: Vec<String>,
}

#[derive(Debug)]
pub struct ResourceNode {
    pub resource_name: String,
    pub ingress_edges: Vec<Rc<Edge>>,
    pub egress_edges: Vec<Rc<Edge>>,
}

#[derive(Debug)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub recipe_name: String,
}

#[derive(Default)]
pub struct Graph {
    pub recipes: HashMap<String, Recipe>,
    pub resources: HashMap<String, Rc<RefCell<ResourceNode>>>,
    pub topological_sort_result: HashMap<String, (u64, u64)>,
}

pub static WORLD_ROOT: &str = "world_root";

impl Graph {
    /// read json recipes, and construct a graph network of resources
    pub fn from_str(file: &str) -> Self {
        // let mut file = File::open(filename).expect("Unable to open JSON file");
        let mut json_data = file.to_string();
        // file.read_to_string(&mut json_data).expect("Unable to read JSON file");
        let recipes_list: Vec<Recipe> =
            serde_json::from_str(&json_data).expect("Error parsing JSON file");
        let mut recipes_table: std::collections::HashMap<String, Recipe> = recipes_list
            .into_iter()
            .map(|r| (r.recipe_name.clone(), r))
            .collect();
        let mut resource_table =
            std::collections::HashMap::<String, Rc<RefCell<ResourceNode>>>::new();

        let mut ret: Graph = Graph::default();
        for (recipe_name, recipe) in &recipes_table {
            ret.add_recipe(recipe);
        }
        ret.topological_sort();
        ret

    }

    //consumes an iterator that can yield &str results, select the names from the current graph,
    //creates a subgraph
    pub fn select<T: IntoIterator<Item=impl AsRef<str>>>(&self, selected_recipes: T) -> Self
    {
        let mut new_graph = Graph::default();
        for recipe in selected_recipes{
            assert!(self.recipes.contains_key(recipe.as_ref()));
            new_graph.add_recipe(&self.recipes[recipe.as_ref()]);
        }
        new_graph.topological_sort();
        new_graph
    }

    pub fn add_recipe(&mut self, recipe: &Recipe) {
        let recipe_name: &String = &recipe.recipe_name;
        self.recipes.insert(recipe_name.clone(), recipe.clone());
        for product in recipe.products.iter() {
            self.add_resource_node(product);
            let mut process_recipes = recipe.resources.clone();
            if process_recipes.is_empty() {
                // The resource comes from the world directly
                // replace the resource with WORLD_ROOT
                process_recipes.push(WORLD_ROOT.to_string());
            }
            for resource in process_recipes.iter() {
                self.add_resource_node(resource);
                let mut from_node = self.resources[resource].borrow_mut();
                let new_edge = Rc::new(Edge {
                    from: resource.clone(),
                    to: product.clone(),
                    recipe_name: recipe_name.clone(),
                });
                from_node.egress_edges.push(new_edge.clone());
                let mut to_node = if resource == product {
                    from_node
                } else {
                    self.resources[product].borrow_mut()
                };
                to_node.ingress_edges.push(new_edge);
            }
        }
    }

    fn add_resource_node(&mut self, product: &String) -> bool {
        if self.resources.contains_key(product.as_str()) { return false; }
        self.resources.insert(
            product.clone(),
            Rc::new(RefCell::new(ResourceNode {
                resource_name: product.clone(),
                ingress_edges: vec![],
                egress_edges: vec![],
            })),
        );
        true
    }

    pub fn topological_sort(&mut self){
        //dfs for topological sort
        let mut pending: HashSet<String> = self.resources.keys().map(|s| s.clone()).collect();
        let mut topology: HashMap<String, (u64, u64)> = HashMap::new();
        //a single resource node must be either in 'pending' or 'topology', not both
        let mut cur = 0_u64;
        //stack implementation to replace recursive program
        while !pending.is_empty() {
            let next = pending.iter().next().unwrap();
            let mut dfs_stack: Vec<String> = Vec::new();
            dfs_stack.push(next.to_string());
            while let Some(s) = dfs_stack.pop() {
                if !topology.contains_key(s.as_str()) {
                    //first time being discovered
                    topology.insert(s.clone(), (cur, u64::MAX));
                    cur += 1;
                    dfs_stack.push(s.clone()); //add it back for second discovery
                    pending.remove(s.as_str());
                    for egress in self.resources[&s].borrow().egress_edges.iter() {
                        if !topology.contains_key(egress.to.as_str()) {
                            dfs_stack.push(egress.to.clone());
                        }
                    }
                } else {
                    //second time being discovered
                    topology.get_mut(s.as_str()).unwrap().1 = cur;
                    cur += 1;
                }
            }
        }
        self.topological_sort_result = topology;
    }

    /// find all the resources that can be produced with the given avaliable resources
    ///
    /// The parameter HashSet is modified in place, so it is changed after the function call
    pub fn expand_coverage(&self, avaliable_resources: &mut HashSet<String>) {
        loop {
            let mut next_iter = false;
            let cur_sources = avaliable_resources.clone();
            for source in cur_sources.iter() {
                for egress in self.resources[source].borrow().egress_edges.iter() {
                    if avaliable_resources.contains(egress.to.as_str()) {
                        continue;
                    };
                    let mut condition_fulfilled = true;
                    for requirement in self.recipes[egress.recipe_name.as_str()].resources.iter() {
                        if !avaliable_resources.contains(requirement.as_str()) {
                            condition_fulfilled = false;
                            break;
                        }
                    }
                    if condition_fulfilled {
                        next_iter = true;
                        avaliable_resources.insert(egress.to.clone());
                    }
                }
            }
            if !next_iter {
                break;
            }
        }
    }

    /// Given a set of products, return all the related recipes and resources.
    ///
    /// Recipes are included if it produces the given product, either directly or indirectly.
    /// Resources are included if it belongs to any of the included recipes
    ///
    /// # Arguments
    ///
    /// * `target_resources` - Any struct that can be converted into a string vector
    ///
    /// # Returns
    ///
    /// `(Vec<String>, Vec<String>)`
    ///
    /// 0. All related resources, sorted in topological order (note: due to the fact DFS uses HashSet, the order is not always the same for different runs. But within one single process, the order is determined)
    /// 1. All related recipes, sorted in alphabetical order
    pub fn find_all_related<'a, T: 'a + Iterator<Item=U>, U: 'a + ToString>(
        &self,
        target_resources: T,
    ) -> (Vec<String>, Vec<String>) {
        let mut pending: Vec<String> = target_resources.map(|u| u.to_string()).collect();
        let mut processed = HashSet::<String>::new();
        let mut related_recipes = HashSet::<String>::new();
        while !pending.is_empty() {
            let cur_item = pending.pop().unwrap();
            if processed.contains(&cur_item) {
                continue;
            }
            //push ingredients
            for edge in self.resources[&cur_item].borrow().ingress_edges.iter() {
                pending.push(edge.from.clone());
                let is_new = related_recipes.insert(edge.recipe_name.clone());
                //if the pushed recipe contains byproduct, push it as well
                if is_new {
                    for byproduct in self.recipes[&edge.recipe_name]
                        .products
                        .iter()
                        .filter(|s| **s != cur_item)
                    {
                        pending.push(byproduct.clone());
                    }
                }
            }
            processed.insert(cur_item);
        }
        let mut resource_vec: Vec<String> = processed.iter().map(|s| s.clone()).collect();
        let mut recipes_vec: Vec<String> = related_recipes.iter().map(|s| s.clone()).collect();
        resource_vec.sort_by_key(|v| u64::MAX - self.topological_sort_result[v].1);
        recipes_vec.sort();
        (resource_vec, recipes_vec)
    }

    pub fn construct_matrix(
        &self,
        recipes: &Vec<String>,
        resources: &Vec<String>,
    ) -> (Array2<f64>, Vec<f64>) {
        let mut matrix: Vec<Vec<f64>> = Vec::new();
        let mut cost_vec: Vec<f64> = Vec::new();

        // Add input for "source nodes" in the graph
        // for (i, sources) in resources.iter().enumerate().filter(|(i, r)| self.resources[*r].borrow().ingress_edges.is_empty()){
        //     let mut new_row = vec![0_f64; resources.len()];
        //     new_row[i] = 1_f64;
        //     matrix.push(new_row);
        //     cost_vec.push(0_f64); // TODO: replace this with corresponding cost of the item
        //     row_names.push(format!("[input] {}", sources));
        // }
        println!("{:?}", resources);
        for recipe in recipes.iter() {
            let mut new_row = vec![0_f64; resources.len()];
            //add negative weights
            for (i, product) in self.recipes[recipe].resources.iter().enumerate() {
                let target_index = resources.iter().position(|v| v == product).unwrap();
                new_row[target_index] -= self.recipes[recipe].resources_rates[i];
            }
            //add positive weights
            for (i, product) in self.recipes[recipe].products.iter().enumerate() {
                let target_index = resources.iter().position(|v| v == product).unwrap();
                new_row[target_index] += self.recipes[recipe].product_rates[i];
            }

            matrix.push(new_row);
            cost_vec.push(self.recipes[recipe].power_consumption);
        }
        (
            Array2::from_shape_vec((matrix.len(), resources.len()), matrix.concat()).unwrap(),
            cost_vec,
        )
    }

    pub fn sort_topologically(&self, resource_list: &mut Vec<impl AsRef<str>>) {
        resource_list.sort_by_key(|v| u64::MAX - self.topological_sort_result[v.as_ref()].1);
    }
}
