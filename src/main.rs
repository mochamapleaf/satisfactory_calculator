use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Deserialize, Serialize)]
struct Recipe {
    recipe_name: String,
    resources: Vec<String>,
    resources_rates: Vec<f32>,
    products: Vec<String>,
    product_rates: Vec<f32>,
    power_consumption: f32,
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



fn main() {
    let (mut recipes, mut resources) = init_graph();
    let mut start_map = Vec::new();
    start_map.push("Adaptive Control Unit".to_string());
    find_dependency(&recipes, &resources, &mut start_map);
}

//read json recipes, and construct a graph network of resources
//return the recipe table, and the graph
fn init_graph() -> (HashMap<String, Recipe>, HashMap<String, Rc<RefCell<ResourceNode>>>) {
    let mut file = File::open("./recipes/recipes1.json").expect("Unable to open JSON file");
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
    return (recipes_table, resource_table);
}

//given recipes and resource table
//find all the resources that can be produced with the given avaliable resources, modify in place
fn expand_coverage(recipes_table: &mut HashMap<String, Recipe>,
                   resources_table: &mut HashMap<String, Rc<RefCell<ResourceNode>>>,
                   avaliable_resources: &mut HashSet<String>) {
    loop{
        let mut next_iter = false;
        let cur_sources = avaliable_resources.clone();
        for source in cur_sources.iter(){
            for egress in resources_table[source].borrow().egress_edges.iter(){
                if avaliable_resources.contains(egress.to.as_str()) { continue; };
                let mut condition_fulfilled = true;
                for requirement in recipes_table[egress.recipe_name.as_str()].resources.iter() {
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

fn find_dependency(recipes_table: &HashMap<String, Recipe>,
                          resources_table: &HashMap<String, Rc<RefCell<ResourceNode>>>,
                          target_resources: &mut Vec<String>) -> HashSet<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut ret : Vec<(Vec<String>, Vec<String>, Vec<String>)>= Vec::new();
    ret.push( (target_resources.clone(), Vec::new(), Vec::new()) );

    loop{
        //expand
        let mut new_solutions = Vec::new();
        let mut exit_loop = true;
        for solution in ret.iter_mut(){
            if solution.0.is_empty() {
                new_solutions.push(solution.clone());
                continue;
            }
            exit_loop = false;
            let target = solution.0.pop().unwrap();
            solution.1.push(target);
            let mut fall_through = true;
            let target = solution.1.last().unwrap();
            let mut new_recipes :HashSet<String> = resources_table[target].borrow().ingress_edges.iter().map(|edge| edge.recipe_name.clone() ).collect();
            for recipe in new_recipes.iter(){
                let mut new_solution = solution.clone();
                new_solution.2.push(recipe.clone());
                for prod in recipes_table[recipe].products.iter(){
                    if let Some(i) = new_solution.0.iter().position(|x| x == prod ){
                        let solution_len = new_solution.0.len();
                        new_solution.0.swap(i, solution_len-1);
                        new_solution.1.push(new_solution.0.pop().unwrap());
                    }
                }
                for resource in recipes_table[recipe].resources.iter(){
                    if new_solution.0.contains(resource) || new_solution.1.contains(resource) { continue; }
                    new_solution.0.push(resource.clone());
                }
                new_solutions.push(new_solution);
                fall_through = false;
            }
            if fall_through{ new_solutions.push(solution.clone());}
        }
        ret = new_solutions;
        ret.iter().for_each(|line| println!("{:?}", line) );
        println!("--------");
        if exit_loop { break; }
    }
    //remove repetitive
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
    ret_set.iter().for_each(|line| println!("{:?}", line) );
    println!("--------");
    return ret_set;
}