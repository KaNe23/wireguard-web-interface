use seed::{prelude::*, *};

mod client_list;

// ------ ------
//     Init
// ------ ------

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    let model = Model::default();
    orders
        .proxy(Msg::ClientList)
        .perform_cmd(async { client_list::Msg::Fetched(client_list::session_request().await) });
    model
}

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

fn view(model: &Model) -> impl IntoNodes<Msg> {
    div![
        style! {
            St::FontFamily => "sans-serif";
            St::MaxWidth => px(650);
            St::Margin => "auto";
        },
        client_list::view(&model.client_list).map_msg(Msg::ClientList),
    ]
}

//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
