use reqwest::*;
use serde_json::{Result, Value};

use serde::{Deserialize,Serialize};

use std::fs::File;
use std::io::Write;

//https://satisfactory.wiki.gg/wiki/Special:ApiSandbox#action=query&format=json&prop=revisions&exportschema=0.11&titles=Template%3ADocsRecipes.json&formatversion=2&rvprop=content&rvslots=main
//https://satisfactory.wiki.gg/wiki/Module:DocsUtils
//https://satisfactory.wiki.gg/wiki/Template:DocsItems.json
//https://satisfactory.wiki.gg/wiki/Template:DocsBuildings.json
//https://satisfactory.wiki.gg/wiki/Template:DocsRecipes.json


#[derive(Debug, Deserialize)]
struct SatisfactoryRecipe{
    #[serde(rename = "className")]
    class_name: String,
    name: String,
    #[serde(rename = "unlockedBy")]
    unlocked_by: String,
    duration: u64,
    ingredients: Vec<SatisfactoryItem>,
    products: Vec<SatisfactoryItem>,
    #[serde(rename = "producedIn")]
    produced_in: Vec<String>,
    #[serde(rename = "inCraftBench")]
    in_craft_bench: bool,
    #[serde(rename = "inWorkshop")]
    in_workshop: bool,
    #[serde(rename = "inBuildGun")]
    in_build_gun: bool,
    #[serde(rename = "inCustomizer")]
    in_customizer: bool,
    #[serde(rename = "manualCraftingMultiplier")]
    manual_crafting_multiplier: f64,
    alternate: bool,
    #[serde(rename = "minPower")]
    min_power: Option<f64>,
    #[serde(rename = "maxPower")]
    max_power: Option<f64>,
    seasons: Vec<String>,
    stable: bool,
    experimental: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Recipe {
    pub recipe_name: String,
    pub resources: Vec<String>,
    pub resources_rates: Vec<f64>,
    pub products: Vec<String>,
    pub product_rates: Vec<f64>,
    pub power_consumption: f64,
    pub production_method: Vec<String>,
    pub unlock_tags: Vec<String>,
}

impl From<SatisfactoryRecipe> for Recipe{
    fn from(source: SatisfactoryRecipe)-> Self{
        let mutiplier = 60_f64 / source.duration as f64;
        let power = match source.produced_in[0].as_str(){
            "Desc_SmelterMk1_C" => 4.0,
            "Desc_ConstructorMk1_C" => 4.0,
            "Desc_AssemblerMk1_C" => 15.0,
            "Desc_FoundryMk1_C" => 16.0,
            "Desc_OilRefineryMk1_C" => 30.0,
            "Desc_Packager_C" => 10.0,
            "Desc_ManufacturerMk1_C" => 55.0,
            "Desc_Blender_C" => 75.0,
            "Desc_GeneratorNuclear_C" => 2500.0, //TODO: prevent this from making all recipes related to
            //nuclear power have a negative loop
            "Desc_HadronCollider_C" => 1000.0, //TODO: Special logic, get average value
            _ => 0.0,
        };
        Recipe {
            recipe_name: source.name,
            resources: source.ingredients.iter().map(|m| m.item.clone()).collect(),
            resources_rates: source.ingredients.iter().map(|m| m.amount * mutiplier).collect(),
            products: source.products.iter().map(|m| m.item.clone()).collect(),
            product_rates: source.ingredients.iter().map(|m| m.amount * mutiplier).collect(),
            power_consumption: power,
            production_method: source.produced_in.clone(),
            unlock_tags: vec![source.unlocked_by],
        }
    }
}

#[derive(Debug, Deserialize)]
struct SatisfactoryItem{
    item: String,
    amount: f64,
}

fn main() {
    //fetch and parse from satisfactory wiki
    let client = reqwest::blocking::Client::new();
    let response = client.get("https://satisfactory.wiki.gg/api.php")
        .query(&[
            ("action", "query"),
            ("format", "json"),
            ("prop", "revisions"),
            ("exportschema", "0.11"),
            ("titles", "Template:DocsRecipes.json"),
            ("formatversion", "2"),
            ("rvprop", "content"),
            ("rvslots", "main"),
        ])
        .send().unwrap();
    let response_text = response.text().unwrap();
    let response_json: Value = serde_json::from_str(response_text.as_str()).unwrap();
    let response_str: String = response_json["query"]["pages"][0]["revisions"][0]["slots"]["main"]["content"].to_string().replace("\\n", "\n").replace("\\\"", "\"");
    let recipe_json_str = response_str.split_once("\n").unwrap().1.rsplit_once("\n").unwrap().0;
    let recipe_json: Value = serde_json::from_str(recipe_json_str).unwrap();
    let recipes : Vec<SatisfactoryRecipe> = recipe_json.as_object().unwrap().iter().map(|(_, value)| serde_json::from_value(value[0].clone()).expect("Error when parsing JSON recipe object") ).collect();
    let mut output = Vec::new();
    for recipe in recipes{
        if recipe.in_workshop | recipe.in_customizer | recipe.in_build_gun{ continue; }
        output.push(Recipe::from(recipe));
    }

    let file = File::create("recipes/auto_output.json").unwrap();
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &output).unwrap();
}