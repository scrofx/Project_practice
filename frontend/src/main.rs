
use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use campuslost_shared::RegisterRequest;

const API_BASE: &str = "http://127.0.0.1:8000";

#[derive(Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

#[function_component(App)]
fn app() -> Html {
    let show_register = use_state(|| false);

    let email_ref = use_node_ref();
    let password_ref = use_node_ref();

    let login_message = use_state(|| None::<(bool, String)>);

    let toggle_auth = {
        let show_register = show_register.clone();

        Callback::from(move |_| {
            show_register.set(!*show_register);
        })
    };

    let full_name_ref = use_node_ref();
    let register_email_ref = use_node_ref();
    let register_password_ref = use_node_ref();

    let register_message = use_state(|| None::<String>);

    // Login request
    let login = {
        let email_ref = email_ref.clone();
        let password_ref = password_ref.clone();
        let login_message = login_message.clone();

        Callback::from(move |_| {
            let Some(email_input) = email_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(password_input) = password_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let payload = LoginRequest {
                email: email_input.value(),
                password: password_input.value(),
            };

            let login_message = login_message.clone();

            spawn_local(async move {
                login_message.set(None);

                let request = Request::post(
                    &format!("{API_BASE}/auth/login")
                )
                    .json(&payload);

                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not create request: {error}")
                        )));
                        return;
                    }
                };

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not connect to server: {error}")
                        )));
                        return;
                    }
                };

                let status = response.status();

                let body = match response.text().await {
                    Ok(body) => body,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not read server response: {error}")
                        )));
                        return;
                    }
                };

                if ! (200..300).contains(&status) {
                    let message = serde_json::from_str::<ApiError>(&body)
                        .map(|error| error.message)
                        .unwrap_or_else(|_| {
                            format!("Login failed (HTTP {status}): {body}")
                        });

                    login_message.set(Some((true, message)));
                    return;
                }

                let auth = match serde_json::from_str::<LoginResponse>(&body) {
                    Ok(auth) => auth,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Invalid server response: {error}")
                        )));
                        return;
                    }
                };

                match LocalStorage::set("campuslost_token", auth.access_token) {
                    Ok(()) => {
                        login_message.set(Some((
                            false,
                            "Login successful! JWT token saved.".to_string()
                        )));
                    }
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not save login token: {error}")
                        )));
                    }
                }
            });
        })
    };

    let register = {
        let full_name_ref = full_name_ref.clone();
        let email_ref = register_email_ref.clone();
        let password_ref = register_password_ref.clone();

        let register_message = register_message.clone();
        let login_message = login_message.clone();
        let show_register = show_register.clone();

        Callback::from(move |_: MouseEvent| {
            let Some(full_name) = full_name_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(email) = email_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(password) = password_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let payload = RegisterRequest {
                full_name: full_name.value(),
                email: email.value(),
                password: password.value(),
                phone: None,
                telegram_handle: None,
                vk_handle: None,
                preferred_contact: None,
            };

            let register_message = register_message.clone();
            let login_message = login_message.clone();
            let show_register = show_register.clone();

            spawn_local(async move {
                register_message.set(None);

                let request = Request::post(
                    &format!("{API_BASE}/auth/register")
                )
                    .json(&payload);

                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        register_message.set(Some(
                            format!("Could not create request: {error}")
                        ));
                        return;
                    }
                };

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        register_message.set(Some(
                            format!("Could not connect to server: {error}")
                        ));
                        return;
                    }
                };

                let status = response.status();

                if !(200..300).contains(&status) {
                    let body = response.text().await.unwrap_or_default();

                    let message = serde_json::from_str::<ApiError>(&body)
                        .map(|error| error.message)
                        .unwrap_or_else(|_| {
                            format!("Registration failed (HTTP {status}): {body}")
                        });

                    register_message.set(Some(message));
                    return;
                }

                show_register.set(false);

                login_message.set(Some((
                    false,
                    "Account created! You can now sign in.".to_string()
                )));
            });
        })
    };

    html! {
        <div class="min-vh-100 bg-light d-flex flex-column">

            <nav class="navbar navbar-dark bg-primary shadow-sm">
                <div class="container">
                    <a class="navbar-brand fw-bold" href="/">
                        {"CampusLost"}
                    </a>
                </div>
            </nav>

            <main class="container flex-grow-1 d-flex
                         justify-content-center align-items-center py-5">

                <div class="card shadow-sm border-0 w-100"
                     style="max-width: 450px;">

                    <div class="card-body p-4 p-md-5">

                        <h2 class="text-center fw-bold mb-2">
                            {
                                if *show_register {
                                    "Create an account"
                                } else {
                                    "Welcome back"
                                }
                            }
                        </h2>

                        <p class="text-center text-muted mb-4">
                            {
                                if *show_register {
                                    "Register to start using CampusLost."
                                } else {
                                    "Sign in to your CampusLost account."
                                }
                            }
                        </p>

                        {
                            if *show_register {
                                html! {
                                    <div>
                                        <div class="mb-3">
                                            <label for="full_name"
                                                   class="form-label fw-semibold">
                                                {"Full name"}
                                            </label>
                                            <input
                                                type="text"
                                                id="full_name"
                                                class="form-control"
                                                placeholder="John Smith"
                                                ref={full_name_ref.clone()}
                                            />
                                        </div>

                                        <div class="mb-3">
                                            <label for="register_email"
                                                   class="form-label fw-semibold">
                                                {"Email address"}
                                            </label>
                                            <input
                                                type="email"
                                                id="register_email"
                                                class="form-control"
                                                placeholder="name@example.com"
                                                ref={register_email_ref.clone()}
                                            />
                                        </div>

                                        <div class="mb-4">
                                            <label for="register_password"
                                                   class="form-label fw-semibold">
                                                {"Password"}
                                            </label>
                                            <input
                                                type="password"
                                                id="register_password"
                                                class="form-control"
                                                placeholder="Create a password"
                                                ref={register_password_ref.clone()}
                                            />
                                        </div>

                                        <button
                                            type="button"
                                            class="btn btn-primary w-100 py-2"
                                            onclick={register}>
                                            {"Create account"}
                                        </button>
                                        {
                                            match &*register_message{
                                                Some(message) => html! {
                                                    <div class="alert alert-danger mt-3" role="alert">
                                                    {message.clone()}
                                                    </div>
                                                },
                                                None => Html::default()
                                            }
                                        }
                                    </div>
                                }
                            } else {
                                html! {
                                    <div>
                                        <div class="mb-3">
                                            <label for="login_email"
                                                   class="form-label fw-semibold">
                                                {"Email address"}
                                            </label>
                                            <input
                                                type="email"
                                                id="login_email"
                                                class="form-control"
                                                placeholder="name@example.com"
                                                ref={email_ref.clone()}
                                            />
                                        </div>

                                        <div class="mb-4">
                                            <label for="login_password"
                                                   class="form-label fw-semibold">
                                                {"Password"}
                                            </label>
                                            <input
                                                type="password"
                                                id="login_password"
                                                class="form-control"
                                                placeholder="Enter your password"
                                                ref={password_ref.clone()}
                                            />
                                        </div>

                                        <button
                                            type="button"
                                            class="btn btn-primary w-100 py-2"
                                            onclick={login}>
                                            {"Sign in"}
                                        </button>

                                        {
                                            match &*login_message {
                                                Some((is_error, message)) => html! {
                                                    <div
                                                        class={if *is_error {
                                                            "alert alert-danger mt-3"
                                                        } else {
                                                            "alert alert-success mt-3"
                                                        }}
                                                        role="alert">
                                                        {message.clone()}
                                                    </div>
                                                },
                                                None => Html::default()
                                            }
                                        }
                                    </div>
                                }
                            }
                        }

                        <hr class="my-4" />

                        <div class="text-center">
                            <span class="text-muted">
                                {
                                    if *show_register {
                                        "Already have an account? "
                                    } else {
                                        "Don't have an account? "
                                    }
                                }
                            </span>

                            <button
                                type="button"
                                class="btn btn-link p-0 text-decoration-none"
                                onclick={toggle_auth}>
                                {
                                    if *show_register {
                                        "Sign in"
                                    } else {
                                        "Register"
                                    }
                                }
                            </button>
                        </div>

                    </div>
                </div>
            </main>

            <footer class="bg-white border-top py-3">
                <div class="container text-center text-muted small">
                    {"CampusLost © 2026"}
                </div>
            </footer>

        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
