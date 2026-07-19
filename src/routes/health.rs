use topcoat::{
    Result,
    router::{RouteFn, route},
};

#[route(GET "/healthz")]
pub async fn health() -> Result<&'static str> {
    Ok("ok")
}

pub fn route_fn() -> RouteFn {
    health.clone()
}
