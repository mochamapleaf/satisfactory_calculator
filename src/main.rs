use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::collections::HashSet;

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
    let mut start_map = HashSet::new();
    start_map.insert("Rotor".to_string());
    let mut recipes_used = HashSet::new();
    recurse_dependency(&recipes, &resources, &mut start_map, &mut recipes_used);
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

fn recurse_dependency(recipes_table: &HashMap<String, Recipe>,
resources_table: &HashMap<String, Rc<RefCell<ResourceNode>>>,
target_resources: &mut HashSet<String>,
recipes_used: &mut HashSet<String>){
    println!("{:?}\n{:?}\n-----", target_resources, recipes_used);
    let targets = target_resources.clone();
    let mut print_out = true;
    for product in targets.iter(){
        target_resources.remove(product);
        let avaliable_recipes: HashSet<String> = resources_table[product]
            .borrow().ingress_edges.iter().map(|edge| edge.recipe_name.clone()).collect();
        for available_recipe in avaliable_recipes.iter() {
            print_out = false;
            if !recipes_used.contains(available_recipe) {
                recipes_used.insert(available_recipe.clone());
                for new_dependency in recipes_table[available_recipe].resources.iter() {
                    target_resources.insert(new_dependency.clone());
                }
                recurse_dependency(recipes_table, resources_table, target_resources, recipes_used);
                recipes_used.remove(available_recipe);
                for new_dependency in recipes_table[available_recipe].resources.iter() {
                    target_resources.remove(new_dependency);
                }
            }else{
                recurse_dependency(recipes_table, resources_table, target_resources, recipes_used);
            }
        }
        target_resources.insert(product.clone());
    }
    if print_out{ println!("resources: {:?}\nrecipes: {:?}", target_resources, recipes_used); }
}