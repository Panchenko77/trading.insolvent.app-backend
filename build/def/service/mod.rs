use endpoint_gen::model::{ProceduralFunction, Service};

mod user_endpoints;

mod auth_endpoints;

/// Returns a vector of the available `Service`s (e.g. `auth`, `user`, `admin`, `chatbot`).
pub fn get_services() -> Vec<Service> {
    vec![
        Service::new("auth", 1, auth_endpoints::get_auth_endpoints()),
        Service::new("user", 2, user_endpoints::get_user_endpoints()),
    ]
}

/// Returns a vector of the available `ProceduralFunction`s (e.g. `auth`, `user`, `admin`, `chatbot`).
pub fn get_proc_functions() -> Vec<ProceduralFunction> {
    vec![]
}
