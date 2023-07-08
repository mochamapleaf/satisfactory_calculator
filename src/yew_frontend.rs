use yew::prelude::*;

//use gloo_net::http::{Request, QueryParams};
use minilp::ComparisonOp;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement, InputEvent, Url}; //Problem, OptimizationDirection,

use std::collections::HashMap;

use crate::graph::*;
use crate::utilities::*;

const DEFAULT_JSON: &str = include_str!("../recipes/auto_output.json");

pub struct App {
    target_resources: Vec<String>,
    target_quanties: Vec<f64>,
    output_recipes: Vec<String>,
    output_quantites: Vec<f64>,
    output_objective: f64,
    lp_mode: ComparisonOp,
    data: Graph,
}

pub enum Msg {
    AddRow,
    RemoveRow(usize),
    UpdateInputResource(usize, String),
    UpdateInputQuantity(usize, f64),
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
        };
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
                let (mut resources, recipes) = self
                    .data
                    .find_all_related(self.target_resources.iter().map(|s| s.as_str()));
                let (mut matrix, mut cost_vec) = self.data.construct_matrix(&recipes, &resources);

                let col_num = matrix.ncols();
                let root_pos = resources.iter().position(|v| v == WORLD_ROOT).unwrap();
                let mut col_selector: Vec<_> = (0..col_num).collect();
                col_selector.remove(root_pos);
                resources.remove(root_pos);
                let matrix = matrix.select(ndarray::Axis(1), &col_selector); //remove empty column "world root"
                let mut matrix_A = matrix.t().to_owned();
                let target_map: HashMap<_, _> = self
                    .target_resources
                    .iter()
                    .zip(self.target_quanties.iter())
                    .collect();
                let mut target_vals: Vec<f64> = resources
                    .iter()
                    .map(|r| **target_map.get(r).unwrap_or(&&0_f64))
                    .collect();
                let (solution, objective) =
                    solve_lp(&matrix_A, &cost_vec, &target_vals, self.lp_mode);
                self.output_objective = objective;
                let temp: Vec<_> = solution
                    .iter()
                    .zip(recipes.iter())
                    .filter(|(&s, _)| s > 10000.0 * f64::EPSILON)
                    .collect();
                self.output_recipes = temp.iter().map(|(_, s)| (*s).clone()).collect();
                self.output_quantites = temp.iter().map(|(&v, _)| v).collect();
            }
        }
        true
    }
    fn view(&self, _ctx: &yew::Context<Self>) -> Html {
        html! {
            <div class="container">
                <SelectionBox selections={vec!["a".to_string(), "b".to_string(), "c".to_string()]} selected=2/>
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
                        <div>{"Total power usage: "} {self.output_objective}</div>
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
    //TODO: move to discard callback function
}

#[function_component]
fn RecipesTableRow(props: &RecipesTableRowProps) -> Html {
    html! {
        <tr style={if props.index & 0b1 == 0{
            "background-color: #f2f2f2;"
            }else{
            "background-color: #ffffff;"
            }}>
        <td>{props.recipe.recipe_name.clone()}</td>
        <td style="verticle-align: middle; text-align: center;">{for (0..props.recipe.resources.len()).map(|i| html!{
                <ValuedItem name={props.recipe.resources[i].clone()} amount={props.recipe.resources_rates[i]} />
            }) }
        {if !props.recipe.resources.is_empty() { html!{
        <div style="verticle-align: middle; display: inline-flex; justify-content: center;">{'\u{2192}'}</div> //unicode right arrow
        }}else{html!{}} }
        {for (0..props.recipe.products.len()).map(|i| html!{
                <ValuedItem name={props.recipe.products[i].clone()} amount={props.recipe.product_rates[i]} />
            }) }
        </td>
        <td>{props.recipe.production_method.clone()}</td>
        <td>{props.amount}</td>
        </tr>
    }
}

#[derive(Properties, PartialEq)]
pub struct SelectionBoxProps {
    selections: Vec<String>,
    selected: usize,
}

#[function_component]
fn SelectionBox(props: &SelectionBoxProps) -> Html {
    html! {
        <div class="selection-container" style="background-color: gray; display: flex">
            {for props.selections.iter().enumerate().map(|(i, opt)|{
                html!{
                    <div class={if i == props.selected {"selected"} else {""}}>{opt}</div>
                }
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ValuedItemProps {
    name: AttrValue,
    amount: f64,
}

#[function_component]
fn ValuedItem(props: &ValuedItemProps) -> Html {
    html! {
        <div style="height: 25px; border: 1px solid #41b349; border-radius: 6px; display: inline-flex; margin: 3px;">
            <div class="valued-item-icon">
                <img src={generate_item_image_link(props.name.as_str())} width="25" height="25" alt={props.name.clone()}/>
            </div>
            <div style="flex-grow: 1; width: 1px; background-color: #41b349;"></div>
            <div style="align-items: center; justify-content: center; padding: 5px; font-family: Arial; display: flex;">
                <span>{
            if props.amount.fract() == 0.0{
                format!("{:.0}", props.amount)
            }else{
                format!("{:.2}", props.amount)
            }}</span>
            </div>
        </div>
    }
}

impl App {
    fn view_input_row(&self, ctx: &yew::Context<Self>, index: usize) -> Html {
        html! {
                <tr>
                    <td>
                        <input
                            type="text"
                            value={self.target_resources[index].clone()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                    let event: Event = e.dyn_into().unwrap_throw();
        let event_target = event.target().unwrap_throw();
        let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
        web_sys::console::log_1(&target.value().into());
                    Msg::UpdateInputResource(index, target.value().to_owned())
                })}
                        />
                    </td>
                    <td>
                        <input
                            type="number"
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
    fn generate_output_table(&self, ctx: &yew::Context<App>) -> Html {
        html! {
            <>
            {for self.output_recipes.iter().zip(self.output_quantites.iter()).enumerate().map(|(i,(k,v))| html!{
                <RecipesTableRow recipe={self.data.recipes[k].clone()} amount={v} index={i}/>
            })}
            </>
        }
    }
}
