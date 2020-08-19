use seed::{self, prelude::*, *};
#[allow(unused_imports)]
use web_sys::console;

#[derive(Default)]
pub struct Model {
    pub last_response: Option<shared::Response>,
    pub session: String,
    pub username: String,
    pub password: String,
    pub wireguard_config: shared::wg_conf::WireGuardConf,
    pub loaded: bool,
    pub current_page: Page,
    pub old_password: String,
    pub password_confirmation: String,
}

pub enum Page {
    EditUser,
    Login,
    WGCong,
}

impl Default for Page {
    fn default() -> Page {
        Page::Login
    }
}

pub enum Msg {
    NoAction,

    LoginRequest,
    LogoutRequest,

    NewPeer,
    UpdatePeerName(usize, String),
    RemovePeer(usize),

    ShowPage(Page),

    OldPasswordChanged(String),
    PasswordChanged(String),
    UsernameChanged(String),
    ConfirmationChanged(String),

    UpdateUser,

    Fetched(fetch::Result<shared::Response>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::NoAction => {}

        Msg::ShowPage(page) => model.current_page = page,

        Msg::UpdatePeerName(i, name) => {
            model.wireguard_config.peers[i].name = name.clone();
            orders.perform_cmd(async move { Msg::Fetched(update_peer_name(i, name).await) });
        }

        Msg::UsernameChanged(s) => model.username = s,
        Msg::PasswordChanged(s) => model.password = s,
        Msg::OldPasswordChanged(s) => model.old_password = s,
        Msg::ConfirmationChanged(s) => model.password_confirmation = s,

        Msg::LoginRequest => {
            model.loaded = false;
            let username = model.username.clone();
            let password = model.password.clone();
            orders.perform_cmd(async { Msg::Fetched(login_request(username, password).await) });

            model.password.clear();
        }

        Msg::LogoutRequest => {
            model.loaded = false;
            model.username.clear();
            orders.perform_cmd(async { Msg::Fetched(logout_request().await) });
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
            let username = model.username.clone();
            let old_password = model.old_password.clone();
            let new_password = model.password.clone();
            let password_confirmation = model.password_confirmation.clone();
            model.loaded = false;
            orders.perform_cmd(async {
                Msg::Fetched(
                    update_user_request(
                        username,
                        old_password,
                        new_password,
                        password_confirmation,
                    )
                    .await,
                )
            });
            model.old_password.clear();
            model.password.clear();
            model.password_confirmation.clear();
        }

        Msg::Fetched(Ok(response_data)) => match response_data {
            shared::Response::LoginSuccess { session } => {
                model.last_response = Some(shared::Response::Success);
                model.loaded = true;
                model.session = session;
                orders.perform_cmd(async { Msg::Fetched(config_request().await) });
            }
            shared::Response::LoginFailure => {
                model.last_response = Some(shared::Response::Failure);
                model.loaded = true;
            }
            shared::Response::WireGuardConf { config } => {
                model.wireguard_config = config;
                model.current_page = Page::WGCong;
                model.loaded = true;
            }
            shared::Response::Logout => {
                model.loaded = true;
                model.session.clear();
                model.current_page = Page::Login;
            }
            shared::Response::Failure => {
                model.last_response = Some(shared::Response::Failure);
                model.loaded = true;
            }
            shared::Response::Success => {
                model.last_response = Some(shared::Response::Success);
                model.loaded = true;
                orders.perform_cmd(async { Msg::ShowPage(Page::WGCong) });
            }
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

async fn update_user_request(
    name: String,
    old_password: String,
    new_password: String,
    password_confirmation: String,
) -> fetch::Result<shared::Response> {
    fetch::Request::new("/api/update_user")
        .method(fetch::Method::Post)
        .json(&shared::Request::UpdateUser {
            name,
            old_password,
            new_password,
            password_confirmation,
        })?
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

fn display_interface(interface: &shared::wg_conf::Interface) -> Vec<Node<Msg>> {
    nodes![li![
        attrs! {At::Class => "list-group-item rounded-0"},
        div![format!("Interface: {}", interface.address.to_string())],
        div![format!("Private Key: {}", interface.private_key)],
        div![format!("Public Key: {}", interface.public_key)],
    ]]
}

fn display_peer(index: usize, peer: &shared::wg_conf::Peer) -> Vec<Node<Msg>> {
    // making lots of copies for all the closures
    let name = peer.name.clone();
    let div_id1 = format!("peer{}", index);
    let div_id2 = div_id1.clone();
    let div_id3 = div_id1.clone();
    let input_id1 = format!("peer{}i", index);
    let input_id2 = input_id1.clone();
    let input_id3 = input_id1.clone();
    nodes![li![
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
            attrs! {At::Id => input_id3, At::Style => "display: none"},
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

                    action = Msg::UpdatePeerName(index, value);
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
            At::Href => format!("api/download_peer/{}", index),
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
                    Msg::RemovePeer(index)
                } else {
                    Msg::NoAction
                }
            }),
            "Remove"
        ],
    ]]
}

fn wg_conf_page(wg_config: &shared::wg_conf::WireGuardConf) -> Vec<Node<Msg>> {
    nodes![
        ul![
            attrs! {At::Class => "list-group", At::Style => "margin-top: -1px !important"},
            display_interface(&wg_config.interface),
            wg_config
                .peers
                .clone()
                .into_iter()
                .enumerate()
                .map(|(i, peer)| { display_peer(i, &peer) })
        ],
        button![
            attrs! {At::Class => "btn btn-secondary mt-1"},
            ev(Ev::Click, |_| Msg::NewPeer),
            "Add New Peer"
        ],
    ]
}

fn edit_user_page(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        div![
            attrs! {At::Class => "span12 mt-0", At::Style => "margin-top: -1px !important"},
            div![
                attrs! {At::Class => "input-group"},
                div![
                    attrs! {At::Class => "input-group-prepend w-25"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0 w-100"},
                        "Username"
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::UsernameChanged),
                    attrs! {
                        At::Value => model.username,
                        At::Type => "text",
                        At::Class => "form-control rounded-0",
                    },
                    id!("user"),
                ],
            ],
            div![
                attrs! {At::Class => "input-group"},
                div![
                    attrs! {At::Class => "input-group-prepend w-25"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0 w-100"},
                        "Old Password"
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::OldPasswordChanged),
                    attrs! {
                        At::Value => model.old_password,
                        At::AutoFocus => AtValue::None,
                        At::Type => "password",
                        At::Class => "form-control rounded-0",
                    },
                    id!("old_password"),
                ],
            ],
            div![
                attrs! {At::Class => "input-group"},
                div![
                    attrs! {At::Class => "input-group-prepend w-25"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0 w-100"},
                        "New Password"
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::PasswordChanged),
                    attrs! {
                        At::Value => model.password,
                        At::Type => "password",
                        At::Class => "form-control rounded-0",
                    },
                    id!("password"),
                ],
            ],
            div![
                attrs! {At::Class => "input-group mt-0", At::Style => "margin-top: -1px !important"},
                div![
                    attrs! {At::Class => "input-group-prepend w-25"},
                    div![
                        attrs! {At::Class => "input-group-text rounded-0 w-100", At::Style => "border-bottom-left-radius: .25rem !important"},
                        "Confirmation"
                    ],
                ],
                input![
                    input_ev(Ev::Input, Msg::ConfirmationChanged),
                    attrs! {
                        At::Value => model.password_confirmation,
                        At::Type => "password",
                        At::Class => "form-control rounded-0",
                        At::Style => "border-bottom-right-radius: .25rem !important"
                    },
                    ev(Ev::KeyDown, |ev| {
                        let ev = ev.dyn_into::<web_sys::KeyboardEvent>().unwrap();
                        if ev.key() == "Enter" {
                            Msg::UpdateUser
                        } else {
                            Msg::NoAction
                        }
                    }),
                    id!("password_confirmation"),
                ],
            ],
        ],
        button![
            attrs! {At::Class => "btn btn-secondary mt-1"},
            ev(Ev::Click, |_| Msg::ShowPage(Page::WGCong)),
            "Back"
        ],
        button![
            attrs! {At::Class => "btn btn-primary mt-1 float-right"},
            ev(Ev::Click, |_| Msg::UpdateUser),
            "Update"
        ],
    ]
}

pub fn view(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        nav_bar(model),
        if !model.loaded {
            nodes![]
        } else {
            match model.current_page {
                Page::Login => login_view(model),
                Page::WGCong => wg_conf_page(&model.wireguard_config),
                Page::EditUser => edit_user_page(&model),
            }
        }
    ]
}

fn find_element_by_id(element_id: &str) -> web_sys::HtmlElement {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    document
        .get_element_by_id(&element_id)
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

fn hide_element(element_id: &str) {
    let element = find_element_by_id(element_id);
    set_style_attribute(element, &"display".to_string(), &"none".to_string());
}

fn show_element(element_id: &str) {
    let element = find_element_by_id(element_id);
    set_style_attribute(element, &"display".to_string(), &"".to_string());
}

fn focus_element(element_id: &str) {
    let _ = find_element_by_id(element_id).focus();
}

fn set_style_attribute(element: web_sys::HtmlElement, attribute: &str, value: &str) {
    let style = element.style();
    let _ = style.set_property(attribute, value);
}

fn display_alert(model: &Model) -> Vec<Node<Msg>> {
    match model.last_response {
        Some(shared::Response::Success) => nodes![div![
            "Success",
            attrs! {At::Class => "alert alert-success float-left p-2 m-0",
            At::Style => "animation: fadeOut 2s forwards;animation-delay: 3s;"}
        ]],
        Some(shared::Response::Failure) => nodes![div![
            "Failure",
            attrs! {At::Class => "alert alert-danger float-left p-2 m-0",
            At::Style => "animation: fadeOut 2s forwards;animation-delay: 3s;"}
        ]],
        _ => nodes![],
    }
}

fn nav_bar(model: &Model) -> Vec<Node<Msg>> {
    nodes![nav![
        attrs! {At::Class => "navbar navbar-light bg-white border rounded-top mt-1"},
        a!["Wireguard", attrs! {At::Class => "navbar-brand"}],
        if !model.loaded {
            nodes![div![attrs![At::Class => "spinner-border text-secondary"]]]
        } else if !model.session.is_empty() {
            nodes![
                display_alert(&model)
                div![
                    span![
                        model.session.clone(),
                        attrs! {At::Class => "btn alert-dark mb-0 mr-2",
                        At::Style => "text-transform: capitalize"},
                        ev(Ev::Click, |_| Msg::ShowPage(Page::EditUser))
                    ],
                    button![
                        attrs! {At::Class => "btn btn-secondary"},
                        ev(Ev::Click, |_| Msg::LogoutRequest),
                        "Logout"
                    ]
                ]
            ]
        } else {
            nodes![display_alert(&model)]
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
                    ev(Ev::KeyDown, |ev| {
                        let ev = ev.dyn_into::<web_sys::KeyboardEvent>().unwrap();
                        if ev.key() == "Enter" {
                            Msg::LoginRequest
                        } else {
                            Msg::NoAction
                        }
                    }),
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
