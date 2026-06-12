use crate::request::RawRequest;
use crate::{response, CronEvent, Error, Payload, Request, Response, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, Read};

pub type HandlerResult<T> = std::result::Result<T, Error>;

pub fn payload_from_json<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).map_err(Error::from)
}

pub fn payload_to_json<T: Serialize>(value: T) -> Result<Vec<u8>> {
    serde_json::to_vec(&value).map_err(Error::from)
}

pub fn run_route<T, F>(handler: F)
where
    T: Payload,
    F: FnOnce(Request<T>) -> HandlerResult<Response>,
{
    let response = read_stdin()
        .and_then(|stdin| serde_json::from_str::<RawRequest>(&stdin).map_err(Error::from))
        .and_then(RawRequest::into_request)
        .map_err(|error| response::bad_request().text(error.to_string()))
        .and_then(|request| {
            handler(request).map_err(|error| response::server_error().text(error.to_string()))
        })
        .unwrap_or_else(|response| response);

    write_response(response);
}

pub fn run_route_empty<F>(handler: F)
where
    F: FnOnce() -> HandlerResult<Response>,
{
    let response = read_stdin()
        .and_then(|_| handler())
        .unwrap_or_else(|error| response::server_error().text(error.to_string()));

    write_response(response);
}

pub fn run_cron<F>(handler: F)
where
    F: FnOnce(CronEvent) -> HandlerResult<Response>,
{
    let response = read_stdin()
        .and_then(|stdin| serde_json::from_str::<CronEvent>(&stdin).map_err(Error::from))
        .and_then(|event| handler(event))
        .unwrap_or_else(|error| response::server_error().text(error.to_string()));

    write_response(response);
}

pub fn run_cron_empty<F>(handler: F)
where
    F: FnOnce() -> HandlerResult<Response>,
{
    let response = read_stdin()
        .and_then(|_| handler())
        .unwrap_or_else(|error| response::server_error().text(error.to_string()));

    write_response(response);
}

fn read_stdin() -> Result<String> {
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;
    Ok(stdin)
}

fn write_response(response: Response) {
    println!(
        "{}",
        serde_json::json!({
            "status": response.status,
            "headers": response.headers,
            "body": response.body,
        })
    );
}
