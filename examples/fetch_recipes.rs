use reqwest::*;
use serde_json::{Result, Value};

use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::Write;

//https://satisfactory.wiki.gg/wiki/Special:ApiSandbox#action=query&format=json&prop=revisions&exportschema=0.11&titles=Template%3ADocsRecipes.json&formatversion=2&rvprop=content&rvslots=main
//https://satisfactory.wiki.gg/wiki/Module:DocsUtils
//https://satisfactory.wiki.gg/wiki/Template:DocsItems.json
//https://satisfactory.wiki.gg/wiki/Template:DocsBuildings.json
//https://satisfactory.wiki.gg/wiki/Template:DocsRecipes.json


#[derive(Debug, Deserialize)]
struct SatisfactoryRecipe {
    #[serde(rename = "className")]
    class_name: String,
    name: String,
    #[serde(rename = "unlockedBy")]
    unlocked_by: String,
    duration: u64,
    ingredients: Vec<SatisfactoryRecipeItemDef>,
    products: Vec<SatisfactoryRecipeItemDef>,
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

#[derive(Debug, Deserialize)]
struct SatisfactoryItem {
    #[serde(rename = "className")]
    class_name: String,
    name: String,
    description: String,
    #[serde(rename = "unlockedBy")]
    unlocked_by: String,
    #[serde(rename = "stackSize")]
    stack_size: u64,
    energy: f64,
    radioactive: f64,
    #[serde(rename = "canBeDiscarded")]
    can_be_discarded: bool,
    #[serde(rename = "sinkPoints")]
    sink_points: u64,
    abbreviation: Option<String>,
    form: String,
    #[serde(rename = "fluidColor")]
    fluid_color: String,
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

impl Recipe {
    //String could be changed to something generic
    fn rename_items<V>(&mut self, rename_map: &std::collections::HashMap<String, V>)
        where
            V: ToString
    {
        self.resources.iter_mut().for_each(|item| {
            if let Some(mapped_name) = rename_map.get(item) {
                *item = mapped_name.to_string();
            }
        });
        self.products.iter_mut().for_each(|item| {
            if let Some(mapped_name) = rename_map.get(item) {
                *item = mapped_name.to_string();
            }
        });
    }
}

impl From<SatisfactoryRecipe> for Recipe {
    fn from(source: SatisfactoryRecipe) -> Self {
        let mutiplier = 60_f64 / source.duration as f64;
        let power = match source.produced_in[0].as_str() {
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
            product_rates: source.products.iter().map(|m| m.amount * mutiplier).collect(),
            power_consumption: power,
            production_method: source.produced_in.clone(),
            unlock_tags: vec![source.unlocked_by],
        }
    }
}

#[derive(Debug, Deserialize)]
struct SatisfactoryRecipeItemDef {
    item: String,
    amount: f64,
}

fn satisfactory_wiki_get_content(page_title: &str) -> String {
    let client = reqwest::blocking::Client::new();
    let response = client.get("https://satisfactory.wiki.gg/api.php")
        .query(&[
            ("action", "query"),
            ("format", "json"),
            ("prop", "revisions"),
            ("exportschema", "0.11"),
            ("titles", page_title),
            ("formatversion", "2"),
            ("rvprop", "content"),
            ("rvslots", "main"),
        ])
        .send().unwrap();
    let response_text = response.text().unwrap();
    let response_json: Value = serde_json::from_str(response_text.as_str()).unwrap();
    response_json["query"]["pages"][0]["revisions"][0]["slots"]["main"]["content"]
        .as_str().unwrap().
        replace("\\n", "\n")
        .replace("\\\"", "\"")
}

fn main() {
    let response_str = satisfactory_wiki_get_content("Template:DocsRecipes.json");
    let recipe_json_str = response_str.split_once("\n").unwrap().1.rsplit_once("\n").unwrap().0; //remove first and last line
    let recipe_json: Value = serde_json::from_str(recipe_json_str).unwrap();
    let recipes: Vec<SatisfactoryRecipe> = recipe_json.as_object().unwrap().iter().map(|(_, value)| serde_json::from_value(value[0].clone()).expect("Error when parsing JSON recipe object")).collect();
    let response_str = satisfactory_wiki_get_content("Template:DocsItems.json");
    let items_json_str = response_str.split_once("\n").unwrap().1.rsplit_once("\n").unwrap().0; //remove first and last line
    let items_json: Value = serde_json::from_str(items_json_str).unwrap();
    let items: Vec<SatisfactoryItem> = items_json.as_object().unwrap().iter().map(|(_, value)| { serde_json::from_value(value[0].clone()).expect("Error when parsing JSON item object") }).collect();
    let classname_map: std::collections::HashMap<String, String> = items.iter().map(|i| (i.class_name.clone(), i.name.clone())).collect();
    let mut output = Vec::new();
    for recipe in recipes {
        if recipe.in_workshop | recipe.in_customizer | recipe.in_build_gun { continue; }
        output.push(Recipe::from(recipe));
    }

    output.iter_mut().for_each(|mut r| r.rename_items(&classname_map));

    //append Ore extractions
    //fetch minable ores
    let client = reqwest::blocking::Client::new();
    let response = client.get("https://satisfactory.wiki.gg/api.php")
        .query(&[
            ("action", "query"),
            ("format", "json"),
            ("list", "categorymembers"),
            ("cmtitle", "Category:Ores"),
        ]).send().unwrap();
    let response_text = response.text().unwrap();
    let response_json: Value = serde_json::from_str(response_text.as_str()).unwrap();
    let mut minable_ores = Vec::new();
    response_json["query"]["categorymembers"].as_array().iter().for_each(|obj| obj.iter().for_each(|v| minable_ores.push(v["title"].as_str().unwrap().replace("\\n", "\n").replace("\\\"", "\"")) ));
    //add coal
    minable_ores.push("Coal".to_string());

    for ore in minable_ores.iter(){
        output.push(Recipe {
            recipe_name: format!("[Mining] {}", ore),
            resources: vec![],
            resources_rates: vec![],
            products: vec![ore.clone()],
            product_rates: vec![60.0],
            power_consumption: 5.0,
            production_method: vec!["Miner Mk.1".to_string()],
            unlock_tags: vec![],
        })
    }

    output.push(Recipe {
        recipe_name: "[Extracting] Crude Oil".to_string(),
        resources: vec![],
        resources_rates: vec![],
        products: vec!["Crude Oil".to_string()],
        product_rates: vec![120.0],
        power_consumption: 40.0,
        production_method: vec!["Oil Extractor".to_string()],
        unlock_tags: vec![],
    });
    output.push(Recipe {
        recipe_name: "[Extracting] Water".to_string(),
        resources: vec![],
        resources_rates: vec![],
        products: vec!["Water".to_string()],
        product_rates: vec![120.0],
        power_consumption: 20.0,
        production_method: vec!["Water Extractor".to_string()],
        unlock_tags: vec![],
    });
    output.push(Recipe {
        recipe_name: "[Extracting] Nitrogen Gas".to_string(),
        resources: vec![],
        resources_rates: vec![],
        products: vec!["Nitrogen Gas".to_string()],
        product_rates: vec![360.0],
        power_consumption: 150.0,
        production_method: vec!["Resource Well Extractors".to_string()],
        unlock_tags: vec![],
    });

    let file = File::create("recipes/auto_output.json").unwrap();
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &output).unwrap();
}