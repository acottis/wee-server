use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
    InvalidMethod,
    InvalidProtocol,
}

#[derive(Debug)]
pub enum StatusCode {
    Ok = 200,
    NoContent = 204,
    NotFound = 404,
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "200 Okay"),
            Self::NoContent => write!(f, "204 No Content"),
            Self::NotFound => write!(f, "404 Not Found"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    Http1_1,
    Http1_0,
    Http0_9,
}

impl From<Protocol> for &str {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Http1_1 => "HTTP/1.1",
            Protocol::Http1_0 => "HTTP/1.0",
            Protocol::Http0_9 => "HTTP/0.9",
        }
    }
}

impl TryFrom<&str> for Protocol {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "http/1.1" => Ok(Self::Http1_1),
            "http/1.0" => Ok(Self::Http1_0),
            "http/0.9" => Ok(Self::Http0_9),
            _ => Err(Error::InvalidProtocol),
        }
    }
}

#[derive(Debug)]
pub enum Method {
    Connect,
    Get,
    Post,
}

impl TryFrom<&str> for Method {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "connect" => Ok(Self::Connect),
            "get" => Ok(Self::Get),
            "post" => Ok(Self::Post),
            _ => Err(Error::InvalidMethod),
        }
    }
}

#[derive(Debug)]
pub struct Response {
    protocol: Protocol,
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Response {
    pub fn new() -> Self {
        Self {
            protocol: Protocol::Http1_1,
            status_code: StatusCode::Ok,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn set_status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    pub fn add_header(
        mut self,
        key: impl ToString,
        value: impl ToString,
    ) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn set_body(mut self, body: impl ToString) -> Self {
        self.body = Some(body.to_string());
        self
    }

    pub fn serialise(&mut self) -> String {
        let protocol: &str = self.protocol.into();
        let status_code = &self.status_code;

        if let Some(body) = &self.body {
            self.headers
                .insert("Content-Length".into(), body.len().to_string());
        }

        let body = self.body.take().unwrap_or("".into());

        let mut headers = String::new();
        self.headers
            .iter()
            .for_each(|(k, v)| headers.push_str(&format!("{k}: {v}\r\n")));

        format!("{protocol} {status_code}\r\n{headers}\r\n{body}",)
    }
}

#[derive(Debug)]
pub struct Request {
    protocol: Protocol,
    method: Method,
    path: String,
    headers: HashMap<String, String>,
    body: String,
    query: Option<HashMap<String, String>>,
}

impl Request {
    pub fn protocol(&self) -> &Protocol {
        &self.protocol
    }
    pub fn method(&self) -> &Method {
        &self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn query(&self) -> &Option<HashMap<String, String>> {
        &self.query
    }
    pub fn body(&self) -> &String {
        &self.body
    }
    pub fn body_mut(&mut self) -> &mut String {
        &mut self.body
    }
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
    pub fn content_len(&self) -> usize {
        self.headers
            .get("content-length")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
    pub fn from_bytes(buf: &[u8]) -> Self {
        let raw_str = std::str::from_utf8(buf).unwrap();
        let (raw_headers, body) = raw_str.split_once("\r\n\r\n").unwrap();
        let mut raw_headers = raw_headers.lines();

        let mut first_line = raw_headers.next().unwrap().split(' ');
        let method = first_line.next().unwrap().try_into().unwrap();
        let mut uri = first_line.next().unwrap().splitn(2, '?');
        let path = uri.next().unwrap().trim_end_matches('/').to_string();
        let query = match uri.next() {
            Some(query) => {
                let mut queries = HashMap::new();
                let query_parts = query.split("&");
                for part in query_parts {
                    let (key, value) = part.split_once("=").unwrap();
                    queries.insert(key.into(), value.into());
                }
                Some(queries)
            }
            None => None,
        };

        let protocol = first_line.next().unwrap().try_into().unwrap();

        let mut headers = HashMap::new();
        raw_headers.for_each(|header| {
            let (key, value) = header.split_once(':').unwrap();
            headers.insert(key.trim().to_lowercase(), value.trim().into());
        });

        let body = body.to_string();

        Self {
            headers,
            body,
            protocol,
            method,
            path,
            query,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respond_to_ping() {
        let request = "POST / HTTP/1.1\r\nHost: 6095-143-159-233-243.ngrok-free.app\r\nUser-Agent: Discord-Interactions/1.0 (+https://discord.com)\r\nContent-Length: 577\r\nContent-Type: application/json\r\nX-Forwarded-Proto: https\r\nX-Signature-Ed25519: 9a10c00a02d8b5d56bf17f3059790c9603a0bba41d8e\r\nAccept-Encoding: gzip\r\n\r\n{\"app_permissions\":\"180224\",\"application_id\":\"1216441490306502796\",\"entitlements\":[],\"id\":\"1218320751015235605\",\"token\":\"foo\",\"type\":1,\"user\":{\"avatar\":\"c6a249645d462\",\"avatar_decoration_data\":null,\"bot\":true,\"discriminator\":\"0000\",\"global_name\":\"Discord\",\"id\":\"6439452\",\"public_flags\":1,\"system\":true,\"username\":\"discord\"},\"version\":1}";

        let http = Request::from_bytes(request.as_bytes());
    }

    #[test]
    fn no_body() {
        let request = "POST / HTTP/1.1\r\n\r\n";
        let http = Request::from_bytes(request.as_bytes());
    }
}
