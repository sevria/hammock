use std::collections::HashMap;

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::response::Response;

/// Per-request context passed to every handler.
///
/// Gives access to the incoming request (method, path, params, query, headers, body)
/// and helpers to build responses.
#[derive(Debug)]
pub struct Context {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    params: HashMap<String, String>,
    body: Bytes,
}

impl Context {
    pub(crate) fn new(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        params: HashMap<String, String>,
        body: Bytes,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            params,
            body,
        }
    }

    // ---- Request ----

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        self.uri.path()
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// A route parameter, e.g. `id` for the route `/users/:id`.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// A percent-decoded query string value, e.g. `page` for `/items?page=2`.
    pub fn query(&self, name: &str) -> Option<String> {
        self.uri.query().and_then(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                    (decode(k) == name).then(|| decode(v))
                })
                .next()
        })
    }

    /// A header value, if present and valid UTF-8.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The raw request body.
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Deserializes the request body as JSON.
    pub fn bind_json<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_slice(&self.body)
    }

    // ---- Response ----

    /// Responds with `200 OK` and a `text/plain` body.
    pub fn string(&self, body: impl Into<String>) -> Response {
        Response::text(body)
    }

    /// Responds with `200 OK` and a `text/html` body.
    pub fn html(&self, body: impl Into<String>) -> Response {
        Response::html(body)
    }

    /// Responds with `200 OK` and a JSON body.
    pub fn json<T: Serialize + ?Sized>(&self, value: &T) -> Response {
        Response::json(value)
    }

    /// Responds with the given status and an empty body.
    pub fn status(&self, status: StatusCode) -> Response {
        Response::new().with_status(status)
    }

    /// Responds with `204 No Content`.
    pub fn no_content(&self) -> Response {
        self.status(StatusCode::NO_CONTENT)
    }

    /// Responds with a `302 Found` redirect.
    pub fn redirect(&self, location: &str) -> Response {
        Response::new()
            .with_status(StatusCode::FOUND)
            .with_header("location", location)
    }
}

/// Percent-decodes a query component, treating `+` as a space.
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match std::str::from_utf8(&bytes[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_query_components() {
        assert_eq!(decode("hello+world"), "hello world");
        assert_eq!(decode("a%20b%2Fc"), "a b/c");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
    }
}
