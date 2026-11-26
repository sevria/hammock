use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use hyper::{HeaderMap, StatusCode};

/// An HTTP response produced by a handler.
#[derive(Debug, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl Response {
    /// Creates an empty `200 OK` response.
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    /// A `text/plain` response.
    pub fn text(body: impl Into<String>) -> Self {
        Self::new()
            .with_content_type("text/plain; charset=utf-8")
            .with_body(body.into())
    }

    /// A `text/html` response.
    pub fn html(body: impl Into<String>) -> Self {
        Self::new()
            .with_content_type("text/html; charset=utf-8")
            .with_body(body.into())
    }

    /// An `application/json` response. Responds with `500` if serialization fails.
    pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => Self::new()
                .with_content_type("application/json")
                .with_body(body),
            Err(_) => Self::new().with_status(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    /// Sets the status code.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Sets a header, replacing any existing value. Invalid names or values are ignored.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    /// Sets the body.
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    fn with_content_type(mut self, value: &'static str) -> Self {
        self.headers
            .insert(CONTENT_TYPE, HeaderValue::from_static(value));
        self
    }

    pub(crate) fn into_hyper(self) -> hyper::Response<Full<Bytes>> {
        let mut res = hyper::Response::new(Full::new(self.body));
        *res.status_mut() = self.status;
        *res.headers_mut() = self.headers;
        res
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

/// Anything a handler can return.
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        Response::new().with_status(self)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::new().with_status(StatusCode::NO_CONTENT)
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, T) {
    fn into_response(self) -> Response {
        self.1.into_response().with_status(self.0)
    }
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(v) => v.into_response(),
            Err(e) => e.into_response(),
        }
    }
}
