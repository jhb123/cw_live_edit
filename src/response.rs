use std::{collections::HashMap, fmt};

pub enum StatusCode {
    // 100
    Continue,
    SwitchingProtocol,
    Processing,
    EarlyHints,
    // 200
    Ok,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultiStatus,
    AlreadyReported,
    ImUsed,
    // 300
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    Unused,
    TemporaryRedirect,
    PermanentRedirect,
    // 400
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenitcationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PrecoditionFailed,
    PayloadTooLarge,
    UriTooLong,
    UnsupportedMediaType,
    RangeNotSatisfiable,
    ExpectationFailed,
    ImATeapot,
    MisdirectRequest,
    UnprocessableContent,
    Locked,
    FailedDependency,
    TooEarly,
    UpgradeRequired,
    PrecoditionRequired,
    TooManyRequests,
    RequestHeaderFieldsTooLarge,
    UnavailableForLegalReasons,
    // 500
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HttpVersionNotSupported,
    VariantAlsoNegotiates,
    InsufficientStorage,
    LoopDetected,
    NotExtended,
    NetworkAuthenticationRequired
}

impl fmt::Display for StatusCode {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            StatusCode::Continue => write!(f,"100 Continue"),
            StatusCode::SwitchingProtocol => write!(f,"101 Switching Protocols"),
            StatusCode::Processing => write!(f,"102 Processing"),
            StatusCode::EarlyHints => write!(f,"103 Early Hints"),
            StatusCode::Ok => write!(f,"200 Ok"),
            StatusCode::Created => write!(f,"201 Created"),
            StatusCode::Accepted => write!(f,"202 Accepted"),
            StatusCode::NonAuthoritativeInformation => write!(f,"203 Non-Authoritative Information"),
            StatusCode::NoContent => write!(f,"204 No Content"),
            StatusCode::ResetContent => write!(f,"205 Reset Content"),
            StatusCode::PartialContent => write!(f,"206 Partial Content"),
            StatusCode::MultiStatus => write!(f,"207 Multi-status"),
            StatusCode::AlreadyReported => write!(f,"208 Already Reported"),
            StatusCode::ImUsed => write!(f,"226 IM used"),
            StatusCode::MultipleChoices => write!(f,"300 Multiple Choices"),
            StatusCode::MovedPermanently => write!(f,"301 Moved Permanently"),
            StatusCode::Found => write!(f,"302 Found"),
            StatusCode::SeeOther => write!(f,"303 See Other"),
            StatusCode::NotModified => write!(f,"304 Not Modified"),
            StatusCode::UseProxy => write!(f,"305 Use Proxy"),
            StatusCode::Unused => write!(f,"306 unused"),
            StatusCode::TemporaryRedirect => write!(f,"307 Temporary Redirect"),
            StatusCode::PermanentRedirect => write!(f,"308 Permanent Redirect"),
            StatusCode::BadRequest => write!(f,"400 Bad Request"),
            StatusCode::Unauthorized => write!(f,"401 Unauthorized"),
            StatusCode::PaymentRequired => write!(f,"402 Payment Required"),
            StatusCode::Forbidden => write!(f,"403 Forbidden"),
            StatusCode::NotFound => write!(f,"404 Not Found"),
            StatusCode::MethodNotAllowed => write!(f,"405 Method Not Allowed"),
            StatusCode::NotAcceptable => write!(f,"406 Not Acceptable"),
            StatusCode::ProxyAuthenitcationRequired => write!(f,"407 Proxy Authenication Required"),
            StatusCode::RequestTimeout => write!(f,"408 Request Timeout"),
            StatusCode::Conflict => write!(f,"409 Conflict"),
            StatusCode::Gone => write!(f,"410 Gone"),
            StatusCode::LengthRequired => write!(f,"411 Length Required"),
            StatusCode::PrecoditionFailed => write!(f,"412 PreconditionFailed"),
            StatusCode::PayloadTooLarge => write!(f,"413 Payload Too Large"),
            StatusCode::UriTooLong => write!(f,"414 URI Too Long"),
            StatusCode::UnsupportedMediaType => write!(f,"415 Unsupported Media Type"),
            StatusCode::RangeNotSatisfiable => write!(f,"416 Range Not Satisfiable"),
            StatusCode::ExpectationFailed => write!(f,"417 Expectation Failed"),
            StatusCode::ImATeapot => write!(f,"418 I'm a teapot"),
            StatusCode::MisdirectRequest => write!(f,"421 Misdirected Request"),
            StatusCode::UnprocessableContent => write!(f,"422 Unprocessable Content"),
            StatusCode::Locked => write!(f,"423 Locked"),
            StatusCode::FailedDependency => write!(f,"424 Failed Dependency"),
            StatusCode::TooEarly => write!(f,"425 Too Early"),
            StatusCode::UpgradeRequired => write!(f,"426 Upgrade Required"),
            StatusCode::PrecoditionRequired => write!(f,"428 Precondition Requred"),
            StatusCode::TooManyRequests => write!(f,"429 Too Many Requests"),
            StatusCode::RequestHeaderFieldsTooLarge => write!(f,"431 Request Header Fields Too Large"),
            StatusCode::UnavailableForLegalReasons => write!(f,"451 Unavailable For Legal Reasons"),
            StatusCode::InternalServerError => write!(f,"500 Internal Server Error"),
            StatusCode::NotImplemented => write!(f,"501 Not Implemented"),
            StatusCode::BadGateway => write!(f,"502 Bad Gateway"),
            StatusCode::ServiceUnavailable => write!(f,"503 Service Unavailable"),
            StatusCode::GatewayTimeout => write!(f,"504 Gateway Timeout"),
            StatusCode::HttpVersionNotSupported => write!(f,"505 HTTP Version Not Supported"),
            StatusCode::VariantAlsoNegotiates => write!(f,"506 Variant Also Negotiates"),
            StatusCode::InsufficientStorage => write!(f,"507 Insufficient Storage"),
            StatusCode::LoopDetected => write!(f,"508 Loop Detected"),
            StatusCode::NotExtended => write!(f,"510 Not Extended"),
            StatusCode::NetworkAuthenticationRequired => write!(f,"511 Network Authentication Required"),
        }
    }

}

pub fn internal_error_response(contents: &str) -> String {
    let length = contents.len();
    format!("HTTP/1.1 {}\r\nContent-Length: {length}\r\n\r\n{contents}", StatusCode::InternalServerError)
}

pub struct ResponseBuilder {
    status_code: Option<StatusCode>,
    headers: Option<HashMap<String,String>>,
    content: Option<String>
}


impl ResponseBuilder  {

    pub fn build(&self) -> String {
        let status_code = if let Some(ref x) = self.status_code { x } else { 
            return internal_error_response("Failed to contruct response. No status code") 
        };
        let headers = if let Some(ref x) = self.headers { x } else { 
            return internal_error_response("Failed to contruct response. No headers") 
        };
        let content = if let Some(ref x) = self.content { x } else { 
            return internal_error_response("Failed to contruct response. No content") 
        };
        let formatted_headers = headers.iter()
            .map(|(k,v )| format!("{k}: {v}") )
            .collect::<Vec<String>>()
            .join("\n");
        format!("HTTP/1.1 {status_code}\r\n{formatted_headers}\r\n\r\n{content}")
    }

    pub fn new() -> Self {
        ResponseBuilder{status_code: None, headers: None, content: None }
    }

    pub fn set_status_code(&mut self, status_code: StatusCode) ->&mut Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn set_html_content(&mut self, content: String) ->& mut Self {
        self.set_content(content, "text/html; charset=utf-8")
    }

    pub fn set_css_content(&mut self, content: String) ->& mut Self {
        self.set_content(content, "text/css; charset=utf-8")
    }

    pub fn set_js_content(&mut self, content: String) ->& mut Self {
        self.set_content(content, "text/javascript; charset=utf-8")
    }

    pub fn set_json_content(&mut self, content: String) ->& mut Self {
        self.set_content(content, "application/json; charset=utf-8")
    }

    pub fn set_content(&mut self, content: String, content_type: &str) ->& mut Self {
        match self.headers {
            Some(ref mut headers) => {
                headers
                    .entry("Content-Type".to_string())
                    .or_insert(content_type.to_string());
                headers
                    .entry("Content-Length".to_string())
                    .or_insert( format!("{}",content.len()));
            },
            None => {
                self.headers = Some(HashMap::from(
                    [
                        ("Content-Type".to_string(),content_type.to_string()),
                        ("Content-Length".to_string(),format!("{}",content.len())),                    
                    ]));
            }
        };
        self.content = Some(content);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::StatusCode;
    use super::ResponseBuilder;

    #[test]
    fn test_server_error_no_status() {
        let response = ResponseBuilder::new().build();
        let expected = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 43\r\n\r\nFailed to contruct response. No status code";
        assert_eq!(&response,expected)
    }

    #[test]
    fn test_server_error_no_headers() {
        let response = ResponseBuilder::new().set_status_code(StatusCode::Ok).build();
        let expected = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 39\r\n\r\nFailed to contruct response. No headers";
        assert_eq!(&response,expected)
    }
}
