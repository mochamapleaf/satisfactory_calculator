use crate::utilities::generate_item_image_link;
use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ValuedItemProps {
    pub name: AttrValue,
    pub amount: f64,
}

#[styled_component(ValuedItem)]
pub fn valued_item(props: &ValuedItemProps) -> Html {
    let container_css = css!{ r#"
        height: 25px;
        border: 1px solid #41b349;
        border-radius: 6px;
        display: inline-flex;
        margin: 3px;
    "#};
    let icon_css = css!{r#"
        height: 100%;
        object-fit: cover;
    "#};
    let split_line_css = css!{r#"
        flex-grow: 1;
        width: 1px;
        background-color: #41b349;
    "#};
    let text_value_css = css!{r#"
        align-items: center;
        justify-content: center;
        padding: 5px;
        display: flex;
    "#};
    html! {
        <div class={container_css}>
            <img class={icon_css} src={generate_item_image_link(props.name.as_str())} alt={props.name.clone()}/>
            <div class={split_line_css}></div>
            <div class={text_value_css}>
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
