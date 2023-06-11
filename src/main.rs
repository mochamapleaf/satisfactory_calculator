mod common_tests;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use ndarray::Array2;
use ndarray_linalg::Inverse;

#[derive(Debug, Deserialize, Serialize)]
struct Recipe {
    recipe_name: String,
    resources: Vec<String>,
    resources_rates: Vec<f64>,
    products: Vec<String>,
    product_rates: Vec<f64>,
    power_consumption: f64,
    production_method: Vec<String>,
    unlock_tags: Vec<String>,
}

#[derive(Debug)]
struct ResourceNode {
    resource_name: String,
    ingress_edges: Vec<Rc<Edge>>,
    egress_edges: Vec<Rc<Edge>>,
}

#[derive(Debug)]
struct Edge {
    from: String,
    to: String,
    recipe_name: String,
}

struct Graph{
    recipes: HashMap<String, Recipe>,
    resources: HashMap<String,Rc<RefCell<ResourceNode>>>,
    topological_sort_result: HashMap<String, (u64, u64)>
}

impl Graph{
    /// read json recipes, and construct a graph network of resources
    fn new(filename: &str) -> Self{
        let mut file = File::open(filename).expect("Unable to open JSON file");
        let mut json_data = String::new();
        file.read_to_string(&mut json_data).expect("Unable to read JSON file");
        let recipes_list: Vec<Recipe> = serde_json::from_str(&json_data).expect("Error parsing JSON file");
        let mut recipes_table: std::collections::HashMap<String, Recipe>
            = recipes_list.into_iter().map(|r| (r.recipe_name.clone(), r)).collect();

        let mut resource_table = std::collections::HashMap::<String, Rc<RefCell<ResourceNode>>>::new();
        for (recipe_name, recipe) in &recipes_table {
            for product in recipe.products.iter() {
                if !resource_table.contains_key(product.as_str()) {
                    resource_table.insert(product.clone(), Rc::new(RefCell::new(
                        ResourceNode {
                            resource_name: product.clone(),
                            ingress_edges: vec![],
                            egress_edges: vec![],
                        })));
                }
                for resource in recipe.resources.iter() {
                    if !resource_table.contains_key(resource.as_str()) {
                        resource_table.insert(resource.clone(), Rc::new(RefCell::new(
                            ResourceNode {
                                resource_name: resource.clone(),
                                ingress_edges: vec![],
                                egress_edges: vec![],
                            })));
                    }
                    let mut from_node = resource_table[resource].borrow_mut();
                    from_node.egress_edges.push(Rc::new(Edge {
                        from: resource.clone(),
                        to: product.clone(),
                        recipe_name: recipe_name.clone(),
                    }));
                    let mut to_node = resource_table[product].borrow_mut();
                    to_node.ingress_edges.push(from_node.egress_edges.last().unwrap().clone());
                }
            }
        }

        //dfs for topological sort
        let mut pending: HashSet<String> = resource_table.keys().map(|s| s.clone()).collect();
        let mut topology: HashMap<String, (u64, u64)> = HashMap::new();
        //a single resource node must be either in 'pending' or 'topology', not both
        let mut cur = 0_u64;
        //stack implementation to replace recursive program
        while !pending.is_empty(){
            let next = pending.iter().next().unwrap();
            let mut dfs_stack: Vec<String> = Vec::new();
            dfs_stack.push(next.to_string());
            while let Some(s) = dfs_stack.pop(){
                if !topology.contains_key(s.as_str()){
                    //first time being discovered
                    topology.insert( s.clone(), (cur, u64::MAX));
                    cur += 1;
                    dfs_stack.push(s.clone()); //add it back for second discovery
                    pending.remove(s.as_str());
                    for egress in resource_table[&s].borrow().egress_edges.iter(){
                        if !topology.contains_key(egress.to.as_str()){
                            dfs_stack.push(egress.to.clone());
                        }
                    }

                }else {
                    //second time being discovered
                    topology.get_mut(s.as_str()).unwrap().1 = cur;
                    cur += 1;
                }
            }
        }

        return Self{
            recipes: recipes_table,
            resources: resource_table,
            topological_sort_result: topology
        };
    }




    /// find all the resources that can be produced with the given avaliable resources
    ///
    /// The parameter HashSet is modified in place, so it is changed after the function call
    fn expand_coverage(&self, avaliable_resources: &mut HashSet<String>){
        loop{
            let mut next_iter = false;
            let cur_sources = avaliable_resources.clone();
            for source in cur_sources.iter(){
                for egress in self.resources[source].borrow().egress_edges.iter(){
                    if avaliable_resources.contains(egress.to.as_str()) { continue; };
                    let mut condition_fulfilled = true;
                    for requirement in self.recipes[egress.recipe_name.as_str()].resources.iter() {
                        if !avaliable_resources.contains(requirement.as_str()){
                            condition_fulfilled = false;
                            break;
                        }
                    }
                    if condition_fulfilled{
                        next_iter = true;
                        avaliable_resources.insert(egress.to.clone());
                    }
                }
            }
            if !next_iter{ break; }
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
    /// 1. All related resources
    /// 2. All related recipes
    fn find_all_related<'a, T:'a + Iterator<Item= U>, U: 'a + ToString>(&self, target_resources: T){
        let mut pending: Vec<String> = target_resources.map(|u| u.to_string()).collect();
        let mut processed = HashSet::<String>::new();
        let mut related_recipes = HashSet::<String>::new();
        loop{
            println!("{:?}", pending);
            if pending.is_empty(){ break; }
            let cur = pending.pop().unwrap();
            if !processed.insert(cur.clone()){}
        }
    }
}

fn main(){
    let mut inst = Graph::new("./recipes/recipes1.json");
    let mut start_map : HashMap<String, f64>= HashMap::new();
    start_map.insert("Plastic".to_string(), 300_f64);
    let dep = inst.find_all_related(start_map.keys().map(|s| s.as_str()));
}


fn main_back() {
    let mut inst = Graph::new("./recipes/recipes1.json");
    let mut start_map : HashMap<String, f64>= HashMap::new();
    //start_map.insert("Plastic".to_string(), 20_f64);
    //start_map.insert("Fuel".to_string(), 300_f64);
    start_map.insert("Plastic".to_string(), 300_f64);
    start_map.insert("Heavy Oil Residue".to_string(), 20_f64);
    let mut start_vec = start_map.iter().map(|(k,_)| k.clone()).collect();
    let dep = find_dependency(&inst.recipes, &inst.resources, &mut start_vec);
    for temp in dep{
        let mut matrix = construct_matrix(&inst.recipes, &inst.resources, &temp.1, &temp.2);
        //insert input and output nodes
        let mut new_io = Vec::new();
        for (i, resource) in temp.1.iter().enumerate(){
            if inst.resources[resource].borrow().ingress_edges.len() == 0{
                //input
                new_io.push(format!("[input]{}", resource));
                let mut temp_vec = vec![0_f64; temp.1.len()];
                temp_vec[i] = 1_f64;
                matrix.push(temp_vec);
            }
        }
        let mut titles = temp.2.clone();
        titles.extend(new_io);
        //construct matrix, find inverse
        matrix.iter().for_each(|v| println!("{:?}", v));
        let mut graph_matrix = Array2::from_shape_vec((titles.len(), titles.len()), matrix.concat()).unwrap();

        let mut graph_inv = graph_matrix.t().inv().unwrap();
        println!("{:?}", graph_matrix);
        let mut output_vec : Vec<f64>= temp.1.iter().map(|v|
            if start_map.contains_key(v){
                start_map[v]
            }else{
                0_f64
            }).collect();
        let mut output_array2 = Array2::from_shape_vec((titles.len(), 1), output_vec ).unwrap();
        let result = graph_inv.dot(&output_array2);
        println!("--------");
        start_map.iter().for_each(|(k,v)|{
            println!("{}[output] {}", k,v);
        });
        println!("--------");
        for i in 0..result.len(){
            println!("{} \t {}", titles[i], result[[i,0]]);
        }




    }
}


fn find_dependency(recipes_table: &HashMap<String, Recipe>,
                          resources_table: &HashMap<String, Rc<RefCell<ResourceNode>>>,
                          target_resources: &mut Vec<String>) -> HashSet<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut ret : Vec<(Vec<String>, Vec<String>, Vec<String>)>= Vec::new();
    // 0. items to be produced
    // 1. items already produced
    // 2. recipes used
    ret.push( (target_resources.clone(), Vec::new(), Vec::new()) );
    let mut branching_items = Vec::new(); //products that cause mutiple solutions

    loop{
        //expand
        let mut new_solutions = Vec::new();
        let mut exit_loop = true;
        for solution in ret.iter_mut(){
            if solution.0.is_empty() {
                new_solutions.push(solution.clone());
                continue;
            }
            exit_loop = false; //only exit loop when all solutions have no more items to be produced
            let target = solution.0.pop().unwrap();
            solution.1.push(target);
            let mut fall_through = true;
            //when there is a new variant solution, clone the current one and add on top of that

            //if there's no variant for this target, the current solution should fall through,
            //with one simple modification: the current target is removed from "to be produced list",
            //and join "already produced list"
            let target = solution.1.last().unwrap();
            let mut new_recipes :HashSet<String> = resources_table[target].borrow().ingress_edges.iter().map(|edge| edge.recipe_name.clone() ).collect();
            let mut variant_count = 0;
            for recipe in new_recipes.iter(){
                let mut new_solution = solution.clone();
                new_solution.2.push(recipe.clone());
                for prod in recipes_table[recipe].products.iter(){
                    //handles when the byproduct is one of the required product
                    if let Some(i) = new_solution.0.iter().position(|x| x == prod ){
                        let solution_len = new_solution.0.len();
                        new_solution.0.swap(i, solution_len-1);
                        new_solution.1.push(new_solution.0.pop().unwrap());
                    }else {
                        //when the byproduct is not one of the required product
                        if !new_solution.1.contains(prod) {
                            new_solution.1.push(prod.clone());
                        }
                    }
                }
                for resource in recipes_table[recipe].resources.iter(){
                    if new_solution.0.contains(resource) || new_solution.1.contains(resource) { continue; }
                    new_solution.0.push(resource.clone());
                }
                new_solutions.push(new_solution);
                variant_count += 1;
                fall_through = false;
            }
            if fall_through{ new_solutions.push(solution.clone());}
            if variant_count > 1 && !branching_items.contains(target) { branching_items.push(target.clone()); }
        }
        ret = new_solutions;
        ret.iter().for_each(|line| println!("{:?}", line) );
        println!("--------");
        if exit_loop { break; }
    }
    //remove repetitive (unnecessary)
    let mut ret_set: HashSet<(Vec<String>, Vec<String>, Vec<String>)> = HashSet::new();
    for sol in ret.into_iter(){
        let mut temp =(
                sol.0.into_iter().collect::<Vec<String>>(),
                sol.1.into_iter().collect::<Vec<String>>(),
                sol.2.into_iter().collect::<Vec<String>>()
        );
        temp.0.sort_unstable();
        temp.1.sort_unstable();
        temp.2.sort_unstable();
        println!("{:?}", temp);
        ret_set.insert(temp);
    }
    println!("--------");
    println!("branching: {:?}", branching_items);
    ret_set.iter().for_each(|line| println!("{:?}", line) );
    println!("--------");
    return ret_set;
}

fn construct_matrix(recipes_table: &HashMap<String, Recipe>,
                    resources_table: &HashMap<String, Rc<RefCell<ResourceNode>>>,
                    path_resources: &Vec<String>,
                    used_recipes: &Vec<String>) -> Vec<Vec<f64>> {
    let mut matrix = Vec::new();
    for recipe in used_recipes{
        let mut row = vec![0_f64; path_resources.len()];
        //add positive weights (products)
        for (i, product) in recipes_table[recipe].products.iter().enumerate(){
            let target_index = path_resources.iter().position(|v| v == product).unwrap();
            row[target_index] = recipes_table[recipe].product_rates[i];
        }
        for (i, resource) in recipes_table[recipe].resources.iter().enumerate() {
            let target_index = path_resources.iter().position(|v| v == resource).unwrap();
            row[target_index] = -recipes_table[recipe].resources_rates[i];
        }
        matrix.push(row);
    }
    matrix
}


