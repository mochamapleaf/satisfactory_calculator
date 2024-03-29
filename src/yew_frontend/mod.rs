use yew::prelude::*;
use yew::callback::Callback;

//use gloo_net::http::{Request, QueryParams};
use minilp::ComparisonOp;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement, InputEvent, Url}; //Problem, OptimizationDirection,

use std::collections::HashMap;

use crate::graph::*;
use crate::utilities::*;


mod filtered_item_selection;
use filtered_item_selection::FilteredItemSelection;
mod valued_item;
use valued_item::ValuedItem;

const DEFAULT_JSON: &str = include_str!("../../recipes/auto_output.json");

pub struct App {
    target_resources: Vec<String>,
    target_quanties: Vec<f64>,
    output_recipes: Vec<String>,
    output_quantites: Vec<f64>,
    output_objective: f64,
    lp_mode: ComparisonOp,
    data: Graph,
    config: HashMap<String, String>,
}

pub enum Msg {
    AddRow,
    RemoveRow(usize),
    UpdateInputResource(usize, String),
    UpdateInputQuantity(usize, f64),
    UpdateConfig(String, String),
    Clear,
    Calculate,
}

impl Component for App {
    type Properties = ();
    type Message = Msg;
    fn create(_ctx: &yew::Context<Self>) -> Self {
        let mut default = App {
            target_resources: vec![],
            target_quanties: vec![],
            output_recipes: vec![],
            output_quantites: vec![],
            output_objective: 0_f64,
            lp_mode: ComparisonOp::Ge,
            data: Graph::from_str(DEFAULT_JSON),
            config: HashMap::new(),
        };
        //set config
        default.config.insert("lp_mode".to_string(), "GreatEq".to_string());
        default.config.insert("cost_func".to_string(), "Power".to_string());
        default.config.insert("recipe_display".to_string(), "Total".to_string());

        //fetch information from url
        let href = web_sys::window().unwrap().location().href().unwrap();
        let url = Url::new(&href).unwrap();
        let params = url.search_params();
        let info = params.get("item").unwrap_or("Plastic".to_string());
        default.target_resources.push(info);
        default.target_quanties.push(1.0);
        _ctx.link().send_message(Msg::Calculate);
        default
    }
    fn update(&mut self, _ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddRow => {
                self.target_resources.push(String::new());
                self.target_quanties.push(1_f64);
            }
            Msg::RemoveRow(index) => {
                if index == usize::MAX {
                    self.target_resources.pop();
                    self.target_quanties.pop();
                } else {
                    self.target_resources.remove(index);
                    self.target_quanties.remove(index);
                }
            }
            Msg::UpdateInputResource(index, new_name) => {
                self.target_resources[index] = new_name;
            }
            Msg::UpdateInputQuantity(index, new_val) => {
                self.target_quanties[index] = new_val;
            }
            Msg::Clear => {
                self.target_quanties.clear();
                self.target_resources.clear();
                self.output_quantites.clear();
                self.output_recipes.clear();
                self.lp_mode = ComparisonOp::Ge;
                self.output_objective = 0_f64;
            }
            Msg::Calculate => {
                //TODO: Feature? Check if the answer from last calculation is reusable
                self.lp_mode = match self.config["lp_mode"].as_str(){
                    "Exact" => ComparisonOp::Eq,
                    _ => ComparisonOp::Ge, //GreatEq
                };

                let (mut resources, recipes) = self
                    .data
                    .find_all_related(self.target_resources.iter().map(|s| s.as_str()));
                let (matrix, cost_vec) = self.data.construct_matrix(&recipes, &resources);

                let col_num = matrix.ncols();
                let root_pos = resources.iter().position(|v| v == WORLD_ROOT).unwrap();
                let mut col_selector: Vec<_> = (0..col_num).collect();
                col_selector.remove(root_pos);
                resources.remove(root_pos);
                let matrix = matrix.select(ndarray::Axis(1), &col_selector); //remove empty column "world root"
                let matrix_a = matrix.t().to_owned();
                let target_map: HashMap<_, _> = self
                    .target_resources
                    .iter()
                    .zip(self.target_quanties.iter())
                    .collect();
                let target_vals: Vec<f64> = resources
                    .iter()
                    .map(|r| **target_map.get(r).unwrap_or(&&0_f64))
                    .collect();
                let (solution, objective) =
                    solve_lp(&matrix_a, &cost_vec, &target_vals, self.lp_mode);
                self.output_objective = objective;
                let mut temp: Vec<_> = solution
                    .iter()
                    .zip(recipes.iter())
                    .filter(|(&s, _)| s > 10000.0 * f64::EPSILON)
                    .collect();
                let temp_graph = self.data.select(temp.iter().map(|(_, s)| s) );
                temp.sort_by_key(|(_, s)| self.data.recipes[*s].resources.iter().map(|v| temp_graph.topological_sort_result[v].1).min().unwrap_or(u64::MAX));
                temp.reverse();
                self.output_recipes = temp.iter().map(|(_, s)| (*s).clone()).collect();
                self.output_quantites = temp.iter().map(|(&v, _)| v).collect();
            },
            Msg::UpdateConfig(key, val) => {self.config.insert(key.clone(), val);
            if &key == "lp_mode" {_ctx.link().send_message(Msg::Calculate);} }
        }
        true
    }
    fn view(&self, _ctx: &yew::Context<Self>) -> Html {
        html! {
            <div class="page-container">
                <div class="config-menu-container">
                    <input type="checkbox" id="settings-menu-button"/>
                    <label for="settings-menu-button" class="config-button">
                        <span class="material-symbols-outlined"></span>
                    </label>
                <h4>{"Mode"}</h4>
                <SelectionBox selections={vec!["GreatEq".to_string(), "Exact".to_string()]} option_callback={
                _ctx.link().callback(|config: String| Msg::UpdateConfig("lp_mode".to_string(), config))
            } selected={self.config["lp_mode"].clone()}/>
                <h4>{"Minimize"}</h4>
                <SelectionBox selections={vec!["Power".to_string(), "Efficiency".to_string(), "Custom".to_string()]} option_callback={
                _ctx.link().callback(|config: String| Msg::UpdateConfig("cost_func".to_string(), config))
            } selected={self.config["cost_func"].clone()}/>
                <h4>{"Recipe Display Mode"}</h4>
                <SelectionBox selections={vec!["Total".to_string(), "Per Machine".to_string()]} option_callback={
                _ctx.link().callback(|config: String| Msg::UpdateConfig("recipe_display".to_string(), config))
            } selected={self.config["recipe_display"].clone()}/>
                </div>
                <div class="content">
                <h1>{"Satisfactory Calculator"}</h1>
                 <div class="button-group">
                            <button onclick={_ctx.link().callback(|_| Msg::AddRow)}>{ "Add Row" }</button>
                            <button onclick={_ctx.link().callback(|_| Msg::RemoveRow(usize::MAX))}>{ "Pop Row" }</button>
                            <button onclick={_ctx.link().callback(|_| Msg::Clear)}>{ "Clear" }</button>
                            <button onclick={_ctx.link().callback(|_| Msg::Calculate)}>{ "Calculate" }</button>
                        </div>
                <div class="row">
                    <div class="column column-left">
                        <table>
                            <thead>
                                <tr>
                                    <th>{"Target resources"}</th>
                                    <th>{"Target quantity"}</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {for (0..self.target_resources.len()).map(|i| self.view_input_row(_ctx, i))}
                            </tbody>
                        </table>

                    </div>
                    <div class="column column-right">
                        <table>
                            <thead>
                                <tr>
                                    <th>{"Recipe name"}</th>
                                    <th>{"Recipe detail"}</th>
                                    <th>{"Production machine"}</th>
                                    <th>{"Machine quantity"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {self.generate_output_table(_ctx)}
                            </tbody>
                        </table>
                        <div>{format!("Total power usage: {:.2}", self.output_objective)}</div>
                    </div>
                </div>
            </div>
            </div>
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct RecipesTableRowProps {
    recipe: Recipe,
    amount: f64,
    index: usize,
    #[prop_or(false)]
    multiplied_amount: bool,
    //TODO: move to discard callback function
}

#[function_component]
fn RecipesTableRow(props: &RecipesTableRowProps) -> Html {
    html! {
        //TODO: Move these style to scss
        <tr style={if props.index & 0b1 == 0{
            "background-color: #f2f2f2;"
            }else{
            "background-color: #ffffff;"
            }}>
        <td>{props.recipe.recipe_name.clone()}</td>
        <td style="verticle-align: middle; text-align: center;">{for (0..props.recipe.resources.len()).map(|i| html!{
                <ValuedItem name={props.recipe.resources[i].clone()} amount={props.recipe.resources_rates[i] * (if props.multiplied_amount {props.amount} else {1.0})} />
            }) }
        {if !props.recipe.resources.is_empty() { html!{
        <div style="verticle-align: middle; display: inline-flex; justify-content: center;">{'\u{2192}'}</div> //unicode right arrow
        }}else{html!{}} }
        {for (0..props.recipe.products.len()).map(|i| html!{
                <ValuedItem name={props.recipe.products[i].clone()} amount={props.recipe.product_rates[i] * (if props.multiplied_amount {props.amount} else {1.0})} />
            }) }
        </td>
        <td>{props.recipe.production_method.clone()}</td>
        <td>{format!("{:.2}",props.amount)}</td>
        </tr>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct SelectionBoxProps {
    selections: Vec<String>,
    option_callback: Callback<String>,
    selected: String,
}

#[function_component]
fn SelectionBox(props: &SelectionBoxProps) -> Html {
    let temp_cb = (props.option_callback).clone();
    html! {
        <div class="selection-container">
            {for props.selections.iter().enumerate().map(|(_, opt)|{
                let onselection = {
                    let click_callback =  temp_cb.clone();
                    let option = opt.clone();
                    Callback::from( move |_| click_callback.emit(option.clone()) )
                };
                html!{
                    <button class={if *opt == props.selected {"option selected"} else {"option"}} onclick={onselection}>
                    {opt}</button>
                }  })
            } //end for
        </div>
    }
}

impl App {
    fn view_input_row(&self, ctx: &yew::Context<Self>, index: usize) -> Html {
        html! {
                <tr>
                    <td>
                        <FilteredItemSelection selections={self.data.resources.keys().map(|v| v.to_string()).collect::<Vec<String>>()}
            selection_callback={ ctx.link().callback(move |selected: String| Msg::UpdateInputResource(index, selected))} />
                    </td>
                    <td>
                        <input
                            type="number"
                            placeholder="Amount"
                            value={self.target_quanties[index].to_string()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                    let event: Event = e.dyn_into().unwrap_throw();
        let event_target = event.target().unwrap_throw();
        let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
        web_sys::console::log_1(&target.value().into());
                    Msg::UpdateInputQuantity(index, target.value().parse().unwrap())
                })}
                        />
                    </td>
                    <td>
                        <button onclick={ctx.link().callback(move |_| Msg::RemoveRow(index)) }>{"Remove"}</button>
                    </td>
                </tr>
            }
    }
    fn generate_output_table(&self, _ctx: &yew::Context<App>) -> Html {
        html! {
            <>
            {for self.output_recipes.iter().zip(self.output_quantites.iter()).enumerate().map(|(i,(k,v))| html!{
                <RecipesTableRow recipe={self.data.recipes[k].clone()} amount={v} index={i} multiplied_amount={self.config["recipe_display"] == "Total"} />
            })}
            </>
        }
    }
}
