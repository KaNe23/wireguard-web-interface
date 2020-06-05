use seed::{self, prelude::*, *};
use wasm_bindgen::*;
use web_sys::console;

#[derive(Default)]
pub struct Model {
    pub session: String,
    pub username: String,
    pub password: String,
    pub wireguard_config: shared::wg_conf::WireGuardConf,
    pub loaded: bool,
}

pub enum Msg {
    LoginRequest,
    LogoutRequest,
    UsernameChanged(String),
    PasswordChanged(String),
    //GetWireGuardConfig,
    NoAction,
    UpdateUser,
    NewPeer,
    RemovePeer(usize),
    Fetched(fetch::ResponseDataResult<shared::Response>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::NoAction => {}

        Msg::UsernameChanged(username) => {
            model.username = username;
        }

        Msg::PasswordChanged(password) => {
            model.password = password;
        }

        Msg::LoginRequest => {
            orders.skip().perform_cmd(login_request(
                model.username.clone(),
                model.password.clone(),
            ));
            model.username.clear();
            model.password.clear();
        }

        Msg::LogoutRequest => {
            orders.skip().perform_cmd(logout_request());
        }

        Msg::NewPeer => {
            orders.skip().perform_cmd(new_peer_request());
        }

        Msg::RemovePeer(index) => {
            orders.skip().perform_cmd(remove_peer_request(index));
        }

        Msg::UpdateUser => {
            //orders.skip().perform_cmd(config_request());
        }

        Msg::Fetched(Ok(response_data)) => match response_data {
            shared::Response::LoginSuccess { session } => {
                model.session = session;
                console::log_1(&format!("{:?}", model.session).into());
                orders.perform_cmd(config_request());
            }
            shared::Response::LoginFailure => {
                model.loaded = true;
            }
            shared::Response::WireGuardConf { config } => {
                console::log_1(&format!("{:?}", config).into());
                model.wireguard_config = config;
                model.loaded = true;
            }
            shared::Response::Logout => model.session.clear(),
        },

        Msg::Fetched(Err(fail_reason)) => {
            log!("Example_A error:", fail_reason);
            orders.skip();
        }
    }
}

async fn login_request(username: String, password: String) -> Result<Msg, Msg> {
    fetch::Request::new("/api/login")
        .method(fetch::Method::Post)
        .send_json(&shared::Request::Login { username, password })
        .fetch_json_data(Msg::Fetched)
        .await
}

async fn logout_request() -> Result<Msg, Msg> {
    fetch::Request::new("/api/logout")
        .method(fetch::Method::Post)
        .fetch_json_data(Msg::Fetched)
        .await
}

pub async fn session_request() -> Result<Msg, Msg> {
    fetch::Request::new("/api/session")
        .method(fetch::Method::Get)
        .fetch_json_data(Msg::Fetched)
        .await
}

async fn config_request() -> Result<Msg, Msg> {
    fetch::Request::new("/api/config")
        .method(fetch::Method::Get)
        .fetch_json_data(Msg::Fetched)
        .await
}

async fn new_peer_request() -> Result<Msg, Msg> {
    fetch::Request::new("/api/new_peer")
        .method(fetch::Method::Get)
        .fetch_json_data(Msg::Fetched)
        .await
}

async fn remove_peer_request(index: usize) -> Result<Msg, Msg> {
    fetch::Request::new(format!("/api/remove_peer/{}", index))
        .method(fetch::Method::Get)
        .fetch_json_data(Msg::Fetched)
        .await
}

pub fn view(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        nav_bar(model),
        if !model.loaded {
            nodes![]
        } else if model.session.is_empty() {
            login_view(model)
        } else {
            nodes![
                ul![
                    attrs! {At::Class => "list-group", At::Style => "margin-top: -1px !important"},
                    li![
                        attrs! {At::Class => "list-group-item rounded-0"},
                        div![format!(
                            "Interface: {}",
                            model.wireguard_config.interface.address.to_string()
                        )],
                        div![format!(
                            "Private Key: {}",
                            model.wireguard_config.interface.private_key
                        )],
                        div![format!(
                            "Public Key: {}",
                            model.wireguard_config.interface.public_key
                        )],
                    ],
                    model
                        .wireguard_config
                        .peers
                        .clone()
                        .into_iter()
                        .enumerate()
                        .map(|(i, peer)| li![
                            attrs! {At::Class => "list-group-item"},
                            div![
                                attrs! {At::Id => format!("peer{}", i)},
                                ev(Ev::Click, move |ev| {
                                    let ele = ev
                                        .target()
                                        .unwrap()
                                        .dyn_into::<web_sys::HtmlDivElement>()
                                        .unwrap();
                                    let _ = show_edit_name(ele);
                                    Msg::NoAction
                                }),
                                format!("Name: {}", peer.name)
                            ],
                            div![format!("Peer: {}", peer.allowed_ips.to_string())],
                            div![format!("Public Key: {}", peer.public_key)],
                            a![
                                attrs! {At::Class => "btn btn-secondary",
                                At::Href => format!("api/download_peer/{}", i),
                                At::Target => "_blank", At::Download => ""},
                                "Download"
                            ],
                            button![
                                attrs! {At::Class => "btn btn-danger float-right"},
                                ev(Ev::Click, move |_| {
                                    if web_sys::window()
                                        .unwrap()
                                        .confirm_with_message("Sure?")
                                        .unwrap()
                                    {
                                        Msg::RemovePeer(i)
                                    } else {
                                        Msg::NoAction
                                    }
                                }),
                                "Remove"
                            ],
                        ])
                ],
                button![
                    attrs! {At::Class => "btn btn-secondary mt-1"},
                    ev(Ev::Click, |_| Msg::NewPeer),
                    "Add New Peer"
                ],
            ]
        }
    ]
}

fn copy_attribute(
    ele1: web_sys::HtmlElement,
    ele2: web_sys::HtmlElement,
    attr: String,
) -> (web_sys::HtmlElement, web_sys::HtmlElement) {
    let value = ele1.get_attribute(&attr).unwrap();
    let _ = ele2.set_attribute(&attr, &value);
    (ele1, ele2)
}

fn show_edit_name(target: web_sys::HtmlDivElement) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    // create a new input field to edit the name and add the old element id to it
    let text_input = document
        .create_element("input")?
        .dyn_into::<web_sys::HtmlElement>()?;
    let (target, text_input) = copy_attribute(target.into(), text_input, "id".to_string());

    let target = target.dyn_into::<web_sys::HtmlElement>()?;
    let text_input = text_input.dyn_into::<web_sys::HtmlInputElement>()?;

    let name = target.inner_html().split_off(6).clone(); // get rid of "Name: "
    text_input.set_value(&name); // prefill the text input with the old name

    // swap the elements
    let list = target.parent_node().unwrap();
    let _ = list.replace_child(&text_input, &target.into())?;

    // set the focus, so one can instantly start typing
    let _ = text_input.focus();

    // create the callback on name confirmation by hitting enter
    let c = Closure::new(move |event: web_sys::KeyboardEvent| {
        let mut new_name = name.clone();
        let target = event
            .target()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();

        if event.key() == "Enter" {
            new_name = target.value();
            // trigger asnyc update in backend
        }

        if event.key() == "Escape" || event.key() == "Enter" {
            let div = document
                .create_element("div")
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            div.set_inner_html(&format!("Name: {}", &new_name));

            let parent = target.parent_node().unwrap();
            let _ = parent.replace_child(&div, &target);

            let (_, div) = copy_attribute(target.into(), div.into(), "id".to_string());

            let c = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
                let ele = ev.target().unwrap();
                let _ = show_edit_name(ele.dyn_into::<web_sys::HtmlDivElement>().unwrap());
            }) as Box<dyn Fn(_)>);

            div.set_onclick(Some(&JsValue::from(c.as_ref()).into()));
            Closure::forget(c);
        }
    });

    text_input.set_onkeyup(Some(&JsValue::from(c.as_ref()).into()));
    Closure::forget(c);
    Ok(())
}

fn nav_bar(model: &Model) -> Vec<Node<Msg>> {
    nodes![nav![
        attrs! {At::Class => "navbar navbar-light bg-white border rounded-top mt-1"},
        a!["Wireguard", attrs! {At::Class => "navbar-brand"}],
        if !model.loaded {
            div![
                attrs![At::Class => "spinner-border text-secondary"],
                span![attrs![At::Class => "sr-only"], "Loading..."],
            ]
        } else {
            if !model.session.is_empty() {
                div![
                    span![
                        model.session.clone(),
                        attrs! {At::Class => "alert alert-dark mb-0 mr-2 p-2",
                        At::Style => "text-transform: capitalize"},
                    ],
                    button![
                        attrs! {At::Class => "btn btn-secondary"},
                        ev(Ev::Click, |_| Msg::LogoutRequest),
                        "Logout"
                    ]
                ]
            } else {
                div![]
            }
        }
    ]]
}

// ok the svg's are overkill, I know
fn login_view(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        div![
            attrs! {At::Class => "span12 mt-0", At::Style => "margin-top: -1px !important"},
            div![
                attrs! {At::Class => "input-group"},
                div![
                    attrs! {At::Class => "input-group-prepend"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0"},
                        svg![
                            attrs! {
                            At::Id => "i-user",
                            At::Xmlns => "http://www.w3.org/2000/svg",
                            At::ViewBox => "0 0 32 32",
                            At::Width => "22",
                            At::Height => "22",
                            At::Fill => "none",
                            At::Stroke => "currentcolor",
                            At::StrokeLinecap => "round",
                            At::StrokeLineJoin => "round",
                            At::StrokeWidth => "3"},
                            path![
                                attrs! {At::D => "M22 11 C22 16 19 20 16 20 13 20 10 16 10 11 10 6 12 3 16 3 20 3 22 6 22 11 Z M4 30 L28 30 C28 21 22 20 16 20 10 20 4 21 4 30 Z"}
                            ]
                        ],
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::UsernameChanged),
                    attrs! {
                        At::Value => model.username,
                        At::AutoFocus => AtValue::None,
                        At::Type => "text",
                        At::Class => "form-control rounded-0",
                        At::Placeholder => "Username"
                    },
                    id!("user"),
                ],
            ],
            div![
                attrs! {At::Class => "input-group mt-0", At::Style => "margin-top: -1px !important"},
                div![
                    attrs! {At::Class => "input-group-prepend"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0", At::Style => "border-bottom-left-radius: .25rem !important"},
                        svg![
                            attrs! {
                            At::Id => "i-key",
                            At::Xmlns => "http://www.w3.org/2000/svg",
                            At::ViewBox => "3 3 25 25",
                            At::Width => "22",
                            At::Height => "22",
                            At::Fill => "none",
                            At::Stroke => "currentcolor",
                            At::StrokeLinecap => "round",
                            At::StrokeLineJoin => "round",
                            At::StrokeWidth => "3"},
                            path![attrs! {At::D => "m 20,10 -6,6 3,3 -3,-3 -4,4 3,3 -3,-3 -2,2"}],
                            circle![attrs! {At::Cx => "22", At::Cy => "8", At::R => "3"}],
                        ],
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::PasswordChanged),
                    attrs! {
                        At::Value => model.password,
                        At::Type => "password",
                        At::Class => "form-control rounded-0",
                        At::Placeholder => "Password",
                        At::Style => "border-bottom-right-radius: .25rem !important"
                    },
                    id!("password"),
                ],
            ],
        ],
        button![
            attrs! {At::Class => "btn btn-secondary mt-1"},
            ev(Ev::Click, |_| Msg::LoginRequest),
            "Login"
        ],
    ]
}
