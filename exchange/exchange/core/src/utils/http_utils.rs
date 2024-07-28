use std::fmt::Display;
use std::str::FromStr;

use bytes::Bytes;
use http::{Request, Uri};

pub type ParamVec = Vec<(String, String)>;

pub struct HttpRequest {
    request: Request<Bytes>,
    uri: Uri,
    parameters: ParamVec,
}
impl HttpRequest {
    pub fn new() -> Self {
        Self {
            request: Default::default(),
            uri: Uri::default(),
            parameters: ParamVec::new(),
        }
    }

    pub fn add_method(&mut self, method: http::Method) {
        *self.request.method_mut() = method
    }

    pub fn add_path(&mut self, uri: Uri) {
        self.uri = uri;
    }

    pub fn set_parameters(&mut self, parameters: ParamVec) {
        self.parameters = parameters;
    }
    #[track_caller]
    pub fn add_parameter(&mut self, key: &'static str, value: impl Display) {
        self.parameters.push((key.into(), value.to_string()));
    }

    pub fn add_header(&mut self, key: impl Display, value: impl Display) {
        self.request.headers_mut().insert(
            http::header::HeaderName::from_str(&key.to_string()).unwrap(),
            http::header::HeaderValue::from_str(&value.to_string()).unwrap(),
        );
    }

    pub fn add_body(&mut self, body: impl Into<Bytes>) {
        *self.request.body_mut() = body.into();
    }
    pub fn no_body(&mut self) {}
    pub fn finish(mut self) -> Request<Bytes> {
        let mut path_and_query = self.uri.path().to_owned().into_bytes();
        path_and_query.extend_from_slice(b"?");

        for (key, val) in self.parameters.as_slice() {
            append_argument_bytes(&mut path_and_query, key, val);
        }
        path_and_query.pop();
        *self.request.uri_mut() = Uri::builder()
            .scheme(self.uri.scheme().expect("scheme").clone())
            .authority(self.uri.authority().unwrap().clone())
            .path_and_query(http::uri::PathAndQuery::try_from(path_and_query).unwrap())
            .build()
            .unwrap();
        self.request
    }
    pub fn get_uri(&self) -> String {
        let mut path_and_query = self.uri.path().to_owned().into_bytes();
        path_and_query.extend_from_slice(b"?");

        for (key, val) in self.parameters.as_slice() {
            append_argument_bytes(&mut path_and_query, key, val);
        }
        path_and_query.pop();
        String::from_utf8(path_and_query).unwrap()
    }
}

/// write `key=val&` to buf if val is not empty
pub fn append_argument_bytes(
    mut buf: impl std::io::Write,
    key: impl AsRef<str>,
    val: impl Display,
) {
    let val = val.to_string();
    if !val.is_empty() {
        write!(buf, "{}={}&", key.as_ref(), val.as_str()).expect("Writing too fast?");
    }
}
/// write `key=val&` to buf if val is not empty
pub fn append_argument_string(buf: &mut String, key: impl AsRef<str>, val: impl Display) {
    let val = val.to_string();
    if !val.is_empty() {
        buf.push_str(key.as_ref());
        buf.push('=');
        buf.push_str(val.as_str());
        buf.push('&');
    }
}

/// write `(key, val)` to buf if val is not empty
pub fn append_argument_pair(buf: &mut ParamVec, key: &'static str, val: impl Display) {
    let val = val.to_string();
    if !val.is_empty() {
        buf.push((key.into(), val.as_str().into()));
    }
}
