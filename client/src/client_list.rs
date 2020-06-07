use seed::{self, prelude::*, *};
#[allow(unused_imports)]
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
    NoAction,
    UpdateUser,
    UpdatePeerName(usize, String),
    NewPeer,
    RemovePeer(usize),
    Fetched(fetch::Result<shared::Response>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::NoAction => {}

        Msg::UpdatePeerName(i, name) => {
            model.wireguard_config.peers[i].name = name.clone();
            orders.perform_cmd(async move { Msg::Fetched(update_peer_name(i, name).await) });
        }

        Msg::UsernameChanged(username) => {
            model.username = username;
        }

        Msg::PasswordChanged(password) => {
            model.password = password;
        }

        Msg::LoginRequest => {
            let username = model.username.clone();
            let password = model.password.clone();
            orders
                .skip()
                .perform_cmd(async { Msg::Fetched(login_request(username, password).await) });

            model.username.clear();
            model.password.clear();
        }

        Msg::LogoutRequest => {
            orders
                .skip()
                .perform_cmd(async { Msg::Fetched(logout_request().await) });
        }

        Msg::NewPeer => {
            orders
                .skip()
                .perform_cmd(async { Msg::Fetched(new_peer_request().await) });
        }

        Msg::RemovePeer(index) => {
            orders
                .skip()
                .perform_cmd(async move { Msg::Fetched(remove_peer_request(index).await) });
        }

        Msg::UpdateUser => {
            orders
                .skip()
                .perform_cmd(async { Msg::Fetched(config_request().await) });
        }

        Msg::Fetched(Ok(response_data)) => match response_data {
            shared::Response::LoginSuccess { session } => {
                model.session = session;
                orders.perform_cmd(async { Msg::Fetched(config_request().await) });
            }
            shared::Response::LoginFailure => {
                model.loaded = true;
            }
            shared::Response::WireGuardConf { config } => {
                model.wireguard_config = config;
                model.loaded = true;
            }
            shared::Response::Logout => model.session.clear(),
            shared::Response::Failure => {}
            shared::Response::Success => {}
        },

        Msg::Fetched(Err(fail_reason)) => {
            log!("error:", fail_reason);
            orders.skip();
        }
    }
}

async fn login_request(username: String, password: String) -> fetch::Result<shared::Response> {
    Request::new("/api/login")
        .method(fetch::Method::Post)
        .json(&shared::Request::Login { username, password })?
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

async fn logout_request() -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/logout")
        .method(fetch::Method::Post)
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

pub async fn session_request() -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/session")
        .method(fetch::Method::Get)
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

async fn config_request() -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/config")
        .method(fetch::Method::Get)
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

async fn new_peer_request() -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/new_peer")
        .method(fetch::Method::Get)
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

async fn remove_peer_request(index: usize) -> fetch::Result<shared::Response> {
    fetch::Request::new(format!("/api/remove_peer/{}", index))
        .method(fetch::Method::Get)
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

async fn update_peer_name(index: usize, name: String) -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/update_peer_name")
        .method(fetch::Method::Post)
        .json(&shared::Request::UpdatePeerName { index, name })?
        .fetch()
        .await?
        .check_status()?
        .json()
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
                        .map(|(i, peer)| {
                            // making lots of copies for all the closures
                            let name = peer.name.clone();
                            let div_id1 = format!("peer{}", i);
                            let div_id2 = div_id1.clone();
                            let div_id3 = div_id1.clone();
                            let input_id1 = format!("peer{}i", i);
                            let input_id2 = input_id1.clone();
                            let input_id3 = input_id1.clone();
                            li![
                                attrs! {At::Class => "list-group-item"},
                                div![
                                    attrs! {At::Id => div_id1},
                                    ev(Ev::Click, move |_ev| {
                                        hide_element(&div_id1);
                                        show_element(&input_id1);
                                        find_element_by_id(&input_id1)
                                            .dyn_into::<web_sys::HtmlInputElement>()
                                            .unwrap()
                                            .set_value(&name); // prefill the text input with the old name

                                        focus_element(&input_id1);
                                        Msg::NoAction
                                    }),
                                    format!("Name: {}", peer.name)
                                ],
                                input![
                                    attrs! {At::Id => format!("peer{}i", i),
                                    At::Style => "display: none"},
                                    ev(Ev::Blur, move |_ev: web_sys::Event| {
                                        hide_element(&input_id3);
                                        show_element(&div_id3);
                                        Msg::NoAction
                                    }),
                                    ev(Ev::KeyDown, move |ev: web_sys::Event| {
                                        let ev = ev.dyn_into::<web_sys::KeyboardEvent>().unwrap();
                                        let mut action = Msg::NoAction;

                                        if ev.key() == "Enter" {
                                            let value = ev
                                                .target()
                                                .unwrap()
                                                .dyn_into::<web_sys::HtmlInputElement>()
                                                .unwrap()
                                                .value();

                                            action = Msg::UpdatePeerName(i.clone(), value);
                                        }

                                        if ev.key() == "Enter" || ev.key() == "Escape" {
                                            hide_element(&input_id2);
                                            show_element(&div_id2);
                                        }

                                        action
                                    })
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
                            ]
                        })
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

fn find_element_by_id(element_id: &String) -> web_sys::HtmlElement {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    document
        .get_element_by_id(&element_id)
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

fn hide_element(element_id: &String) {
    let element = find_element_by_id(element_id);
    set_style_attribute(element, &"display".to_string(), &"none".to_string());
}

fn show_element(element_id: &String) {
    let element = find_element_by_id(element_id);
    set_style_attribute(element, &"display".to_string(), &"".to_string());
}

fn focus_element(element_id: &String) {
    let _ = find_element_by_id(element_id).focus();
}

fn set_style_attribute(element: web_sys::HtmlElement, attribute: &String, value: &String) {
    let style = element.style();
    let _ = style.set_property(attribute, value);
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
