use stylist::yew::styled_component;
use yew::prelude::*;
use web_sys::{Event, HtmlInputElement, InputEvent, HtmlElement};
use crate::utilities::generate_item_image_link;
use wasm_bindgen::{JsCast, UnwrapThrowExt, closure::Closure};

#[derive(PartialEq, Properties)]
pub struct FilteredItemSelectionProps {
    pub selections: Vec<String>,
    pub selection_callback: Callback<String>,
}

#[styled_component(FilteredItemSelection)]
pub fn filtered_item_selection(props: &FilteredItemSelectionProps) -> Html {
    let input_text = use_state(|| String::new());
    let dropdown_visible = use_state(|| false);
    let node_ref = use_node_ref();

    let container_css = css!{r#"
        background: transparent;
        width: 200px;
        position: relative;
    "#};

    let input_text_css = css!{r#"
        background: transparent;
        border: none;
        border-bottom: solid 1px black;
        outline: none;
        height: 30px;
        width: 100%;
        box-sizing: border-box;
        font-size: 18px;
        padding: 0 10px;
        display: inline-flex;
        &:focus {
            background: rgba(0,0,0,0.05);
        }
    "#};

    let dropdown_menu_css = css!{r#"
        display: none;
        left: 0;
        width: 100%;
        position: absolute;
        background-color: white;
        padding: 0;
        max-height: calc(50vh);
        overflow-y: auto;
    "#};

    let ul_css = css!{r#"
        list-style-type: none;
        padding: 0;
        margin: 0;
        width: 100%;
        height: 100%;
        & li{
            display: flex;
            align-items: center;
            justify-content: left;
            height: 30px;
        }
        & li:hover{
            background-color: lightcyan;
        }
    "#};
    let dropdown_status = dropdown_visible.clone();
    let cloned_node_ref = node_ref.clone();
    use_effect_with_deps( move |_| {
        let handle_click_outside = Closure::wrap(Box::new(move |e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlElement>().unwrap();
            //check if clicked target is outside current element root
            if !cloned_node_ref.cast::<HtmlElement>().unwrap().contains(Some(&target)) {
                dropdown_status.set(false);
            }
        }) as Box<dyn FnMut(Event)>);
        web_sys::window().unwrap().document().unwrap().body().unwrap().add_event_listener_with_callback("click", handle_click_outside.as_ref().unchecked_ref()).unwrap();
        handle_click_outside.forget();
    }, ());

    let cb_clone = props.selection_callback.clone();
    let oninput = {
        let text_val = input_text.clone();
        Callback::from(move |e: InputEvent| {
            let target = e.dyn_into::<Event>().unwrap_throw().target().unwrap_throw().dyn_into::<HtmlInputElement>().unwrap_throw();
            text_val.set(target.value().to_owned());
        })
    };

    let onclick = |change_to: &String| {
        let text_val = input_text.clone();
        let dropdown_status = dropdown_visible.clone();
        let change_to = change_to.to_string();
        let select_callback = cb_clone.clone();
        Callback::from(move |_: MouseEvent|{
            text_val.set(change_to.clone());
            dropdown_status.set(false);
            select_callback.emit(change_to.clone());
        })
    };

    let onfocus = {
        let dropdown_status = dropdown_visible.clone();
        Callback::from(move |_| dropdown_status.set(true))
    };

    html! {
        <div class={container_css} ref={node_ref}>
            <input type="text" class={input_text_css} {oninput} value={(*input_text).clone()} {onfocus} />
            <div style={if *dropdown_visible {"display: block;"} else {""}} class={dropdown_menu_css}>
                <ul class={ul_css}>
                    <li onclick={onclick(&"".to_string())}></li>
                    {for props.selections.iter().filter(|&v| v.starts_with(&*input_text) ).map(|option| html!{
                        //TODO: filter out world_root
                        //TODO: Use editing distance to filter
                        //TODO: Add Clear Button
                        <li onclick={onclick(option)}>
                            <img style="height: 80%; margin-right: 10px;" src={generate_item_image_link(option.as_str())} alt={option.clone()}/>
                            {option.clone()}
                        </li>
                    })}
                </ul>
            </div>
        </div>
    }
}
