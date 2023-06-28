mod tests;

mod yew_frontend;
mod graph;
mod utilities;

use crate::yew_frontend::*;

fn main(){
    yew::Renderer::<App>::new().render();
}
