use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

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
                    recipe_name: recipe_name.clone() }));
                let mut to_node = resource_table[product].borrow_mut();
                to_node.ingress_edges.push(from_node.egress_edges.last().unwrap().clone());
            }
        }
    }
    println!("{}, {:?}", "Iron Ingot\\", resource_table["Iron Ingot"]);
}
