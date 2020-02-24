use seed::{prelude::*, *};

mod client_list;

// ------ ------
//     Model
// ------ ------

#[derive(Default)]
struct Model {
    client_list: client_list::Model,
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    ClientList(client_list::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::ClientList(msg) => {
            client_list::update(
                msg,
                &mut model.client_list,
                &mut orders.proxy(Msg::ClientList),
            );
        }
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> impl View<Msg> {
    div![
        style! {
            St::FontFamily => "sans-serif";
            St::MaxWidth => px(650);
            St::Margin => "auto";
        },
        client_list::view(&model.client_list).map_msg(Msg::ClientList),
    ]
}

fn after_mount(_: Url, orders: &mut impl Orders<Msg>) -> AfterMount<Model> {
    let model: Model = Default::default();
    orders
        .proxy(Msg::ClientList)
        .perform_cmd(client_list::session_request());
    AfterMount::new(model)
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::builder(update, view)
        .after_mount(after_mount)
        .build_and_start();
}
