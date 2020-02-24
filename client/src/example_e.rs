use seed::browser::service::fetch;
use seed::{prelude::*, *};
use std::borrow::Cow;
use std::mem;
use wasm_bindgen::JsCast;
use web_sys::{
    self,
    console::{log_1, log_2},
    File, FormData,
};

pub const TITLE: &str = "Example E";
pub const DESCRIPTION: &str =
    "Fill form and click 'Submit` button. Server echoes the form back. See console log for more info.";

fn get_request_url() -> impl Into<Cow<'static, str>> {
    "/api/form"
}

// ------ ------
//     Model
// ------ ------

#[derive(Default, Debug)]
pub struct Form {
    title: String,
    description: String,
    file: Option<File>,
    answer: bool,
}

impl Form {
    fn to_form_data(&self) -> Result<web_sys::FormData, JsValue> {
        let form_data = web_sys::FormData::new()?;
        form_data.append_with_str("title", &self.title)?;
        form_data.append_with_str("description", &self.description)?;
        if let Some(file) = &self.file {
            form_data.append_with_blob("file", file)?;
        }
        form_data.append_with_str("answer", &self.answer.to_string())?;
        Ok(form_data)
    }
}

pub enum Model {
    ReadyToSubmit(Form),
    WaitingForResponse(Form),
}

impl Default for Model {
    fn default() -> Self {
        Self::ReadyToSubmit(Form {
            title: "I'm title".into(),
            description: "I'm description".into(),
            file: None,
            answer: true,
        })
    }
}

impl Model {
    fn form(&self) -> &Form {
        match self {
            Self::ReadyToSubmit(form) | Self::WaitingForResponse(form) => form,
        }
    }
    fn form_mut(&mut self) -> &mut Form {
        match self {
            Self::ReadyToSubmit(form) | Self::WaitingForResponse(form) => form,
        }
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    TitleChanged(String),
    DescriptionChanged(String),
    FileChanged(Option<File>),
    AnswerChanged,
    FormSubmitted(String),
    ServerResponded(fetch::ResponseDataResult<String>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::TitleChanged(title) => model.form_mut().title = title,
        Msg::DescriptionChanged(description) => model.form_mut().description = description,
        Msg::FileChanged(file) => {
            model.form_mut().file = file;
        }
        Msg::AnswerChanged => toggle(&mut model.form_mut().answer),
        Msg::FormSubmitted(id) => {
            let form = take(model.form_mut());
            orders.perform_cmd(send_request(form.to_form_data().unwrap()));
            *model = Model::WaitingForResponse(form);
            log!(format!("Form {} submitted.", id));
        }
        Msg::ServerResponded(Ok(response_data)) => {
            *model = Model::ReadyToSubmit(Form::default());
            clear_file_input();
            log_2(
                &"%cResponse data:".into(),
                &"background: yellow; color: black".into(),
            );
            log_1(&response_data.into());
        }
        Msg::ServerResponded(Err(fail_reason)) => {
            *model = Model::ReadyToSubmit(take(model.form_mut()));
            error!("Request failed!", fail_reason);
        }
    }
}

async fn send_request(form: FormData) -> Result<Msg, Msg> {
    fetch::Request::new(get_request_url())
        .method(fetch::Method::Post)
        .body(form.into())
        .fetch_string_data(Msg::ServerResponded)
        .await
}

#[allow(clippy::option_map_unit_fn)]
fn clear_file_input() {
    seed::document()
        .get_element_by_id("form-file")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|file_input| {
            // Note: `file_input.set_files(None)` doesn't work
            file_input.set_value("")
        });
}

fn take<T: Default>(source: &mut T) -> T {
    mem::replace(source, T::default())
}

fn toggle(value: &mut bool) {
    *value = !*value
}

// ------ ------
//     View
// ------ ------

fn view_form_field(mut label: Node<Msg>, control: Node<Msg>) -> Node<Msg> {
    label.add_style("margin-right", unit!(7, px));
    div![
        style! {
          "margin-bottom" => unit!(7, px),
          "display" => "flex",
        },
        label,
        control
    ]
}

pub fn view(model: &Model, intro: impl FnOnce(&str, &str) -> Vec<Node<Msg>>) -> Vec<Node<Msg>> {
    let btn_disabled = match model {
        Model::ReadyToSubmit(form) if !form.title.is_empty() => false,
        _ => true,
    };

    let form_id = "A_FORM".to_string();
    let form = form![
        style! {
            St::Display => "flex",
            St::FlexDirection => "column",
        },
        ev(Ev::Submit, move |event| {
            event.prevent_default();
            Msg::FormSubmitted(form_id)
        }),
        view_form_field(
            label!["Title:", attrs! {At::For => "form-title" }],
            input![
                input_ev(Ev::Input, Msg::TitleChanged),
                attrs! {
                    At::Id => "form-title",
                    At::Value => model.form().title,
                    At::Required => true.as_at_value(),
                }
            ]
        ),
        view_form_field(
            label!["Description:", attrs! {At::For => "form-description" }],
            textarea![
                input_ev(Ev::Input, Msg::DescriptionChanged),
                attrs! {
                    At::Id => "form-description",
                    At::Value => model.form().description,
                    At::Rows => 1,
                },
            ],
        ),
        view_form_field(
            label!["Text file:", attrs! {At::For => "form-file" }],
            input![
                ev(Ev::Change, |event| {
                    let file = event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .and_then(|file_input| file_input.files())
                        .and_then(|file_list| file_list.get(0));

                    Msg::FileChanged(file)
                }),
                attrs! {
                    At::Type => "file",
                    At::Id => "form-file",
                    At::Accept => "text/plain",
                }
            ],
        ),
        view_form_field(
            label!["Do you like cocoa?:", attrs! {At::For => "form-answer" }],
            input![
                ev(Ev::Click, |_| Msg::AnswerChanged),
                attrs! {
                    At::Type => "checkbox",
                    At::Id => "form-answer",
                    At::Checked => model.form().answer.as_at_value(),
                }
            ],
        ),
        button![
            style! {
                "padding" => format!{"{} {}", px(2), px(12)},
                "background-color" => if btn_disabled { CSSValue::Ignored } else { "aquamarine".into() },
            },
            attrs! {At::Disabled => btn_disabled.as_at_value()},
            "Submit"
        ]
    ];

    nodes![intro(TITLE, DESCRIPTION), form]
}
