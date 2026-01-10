use wstd::http::{Body, Request, request};

pub trait JsonRequest {
    fn json<T: serde::Serialize>(self, value: &T) -> Result<Request<Body>, anyhow::Error>;
}

impl JsonRequest for request::Builder {
    fn json<T: serde::Serialize>(self, value: &T) -> Result<Request<Body>, anyhow::Error> {
        Ok(self
            .header("content-type", "application/json")
            .body(Body::from_json(value)?)?)
    }
}
