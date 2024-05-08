use std::{collections::HashMap, fmt::{self, Display}};


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

impl Display for StatusCode {

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
    unique_headers: HashMap<String,String>,
    headers: Vec<(String,String)>,
    content: String
}


impl ResponseBuilder  {

    pub fn build(&mut self) -> String {
        let status_code = if let Some(ref x) = self.status_code { x } else { 
            return internal_error_response("Failed to contruct response. No status code") 
        };

        if self.headers.is_empty() && self.unique_headers.is_empty() { 
            return internal_error_response("Failed to contruct response. No headers") 
        };

        let mut headers = self.unique_headers.iter()
            .map(|(k,v)| (k.to_owned(),v.to_owned()) )
            .collect::<Vec<(String,String)>>();

        headers.extend(self.headers.iter().map(|x| x.to_owned()));

        let formatted_headers = headers.iter()
            .map(|(k,v )| format!("{k}: {v}") )
            .collect::<Vec<String>>()
            .join("\n");

        format!("HTTP/1.1 {status_code}\r\n{formatted_headers}\r\n\r\n{}",self.content)
    }

    pub fn new() -> Self {
        ResponseBuilder{status_code: None, unique_headers: HashMap::new(), headers: Vec::new(), content: "".to_string() }
    }

    pub fn set_status_code(&mut self, status_code: StatusCode) ->&mut Self {
        self.status_code = Some(status_code);
        self
    }
    
    pub fn set_text_content(&mut self, content: String) ->& mut Self {
        self.set_content(content, "text/plain; charset=utf-8")
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
        self.unique_headers
            .entry("Content-Type".to_string())
            .or_insert(content_type.to_string());
        self.unique_headers
            .entry("Content-Length".to_string())
            .or_insert( format!("{}",content.len()));
        self.content = content;
        self
    }

    pub fn add_cookie(&mut self, cookie: SetCookie<String>) -> & mut Self {
        
        let cookie = ("Set-Cookie".to_string(), format!("{cookie}"));
        self.headers.push(cookie);
        self        
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseSameSiteError;

impl std::str::FromStr for SameSite {
    type Err = ParseSameSiteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Strict" => Ok(Self::Strict),
            "Lax" => Ok(Self::Lax),
            "None" => Ok(Self::None),
            _ => Err(ParseSameSiteError),
        }
    }
}

impl Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SameSite::Strict => write!(f, "Strict"),
            SameSite::Lax => write!(f, "Lax"),
            SameSite::None => write!(f, "None"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SetCookie<T: Display> {
    pub name: String,
    pub value: T,
    pub domain: Option<String>,
    pub expires: Option<chrono::NaiveDateTime>,
    pub http_only: Option<bool>,
    pub max_age: Option<chrono::Duration>,
    pub partitioned: Option<bool>,
    pub path: Option<String>,
    pub same_site: Option<SameSite>,
    pub secure: Option<bool>,
}

impl<T: Display> SetCookie<T> {
    pub fn new(name: String, value: T) -> Self {
        SetCookie {
            name,
            value,
            domain: None,
            expires: None,
            http_only: None,
            max_age: None,
            partitioned: None,
            path: None,
            same_site: None,
            secure: None,
        }
    }

    pub fn set_max_age(&mut self, max_age: chrono::Duration) -> &Self {
        self.max_age = Some(max_age);
        self
    }
    pub fn set_expires(&mut self, expires: chrono::NaiveDateTime) -> &Self {
        self.expires = Some(expires);
        self
    }
}

impl<T: Display> Display for SetCookie<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.value)?;
        if let Some(s) = &self.domain {
            write!(f, "; Domain={}", s)?;
        }
        if let Some(s) = &self.expires {
            let t = chrono::NaiveDateTime::format(&s, "%a, %d %b %Y %X GMT");
            write!(f, "; Expires={}", t)?;
        }
        if let Some(s) = &self.http_only {
            if *s {write!(f, "; HttpOnly")?};
        }
        if let Some(s) = &self.max_age {
            write!(f, "; Max-Age={}", s.num_seconds())?;
        }
        if let Some(s) = &self.partitioned {
            if *s {write!(f, "; Partitioned")?}
        }
        if let Some(s) = &self.path {
            write!(f, "; Path={}", s)?;
        }
        if let Some(s) = &self.same_site {
            write!(f, "; SameSite={}", s)?;
        }
        if let Some(s) = &self.secure {
            if *s {write!(f, "; Secure")?}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate};
    use super::SetCookie;
    use super::StatusCode;
    use super::ResponseBuilder;

    fn set_cookie_from_header_text(header_text: &str) -> Result<SetCookie<String>, String> {
        let mut header_info = header_text.split(";");
        let mut name_val = header_info.next().unwrap().split("=");
        let name = name_val.next().unwrap();
        let val = name_val.next().unwrap_or("");
    
        let mut cookie = SetCookie::new(name.to_string(), val.to_string());
    
        header_info.try_for_each(|x| {
            match x {
                "HttpOnly" => {
                    cookie.http_only = Some(true);
                    Ok(())
                }
                "Partitioned" => {
                    cookie.partitioned = Some(true);
                    Ok(())
                }
                "Secure" => {
                    cookie.secure = Some(true);
                    Ok(())
                }
                x if x.contains("=") => {
                    // these unwraps are fine. The string contains at least
                    // 1 equals, so it will always have at 2 parts.
                    let mut iter = x.split("=");
                    let k = iter.next().unwrap();
                    let v = iter.next().unwrap().to_string();
                    match k.trim() {
                        "Domain" => {
                            cookie.domain = Some(v);
                            Ok(())
                        }
                        "Expires" => {
                            match chrono::NaiveDateTime::parse_from_str(&v, "%a, %d %b %Y %X GMT") {
                                Ok(x) => {
                                    cookie.expires = Some(x);
                                    Ok(())
                                }
                                Err(e) => Err(format!("{e}, {v} is not a valid date format")),
                            }
                        }
                        "Max-Age" => {
                            let t = match v.parse(){
                                Ok(t) => t,
                                Err(_) => return Err("cannot parse Max-Age as an integer".to_string()),
                            };
                            cookie.max_age = Some(Duration::seconds(t));
                            Ok(())
                        }
                        "Path" => {
                            cookie.path = Some(v.to_string());
                            Ok(())
                        }
                        "SameSite" => match SameSite::from_str(&v) {
                            Ok(same_site_val) => {
                                cookie.same_site = Some(same_site_val);
                                Ok(())
                            }
                            Err(_) => Err(format!("Same site cannot be {v}")),
                        },
                        k => Err(format!("{k} is not a valid parameter for a cookie")),
                    }
                }
                x => Err(format!("{x} is not a valid parameter for a cookie")),
            }
        })?;
    
        Ok(cookie)
    }

    #[test]
    fn test_simple_cookie_name() {
        let cookie = set_cookie_from_header_text("id=a3fWa").unwrap();
        assert_eq!(cookie.name, "id");
    }

    #[test]
    fn test_simple_cookie_value() {
        let cookie = set_cookie_from_header_text("id=a3fWa").unwrap();
        assert_eq!(cookie.value, "a3fWa");
    }

    #[test]
    fn test_cookie_expire_time() {
        let cookie = set_cookie_from_header_text("id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:01 GMT").unwrap();
        assert_eq!(
            cookie.expires.unwrap(),
            NaiveDate::from_ymd_opt(2015, 10, 21)
                .unwrap()
                .and_hms_opt(7, 28, 1)
                .unwrap()
        );
    }

    #[test]
    fn test_max_age() {
        let cookie = set_cookie_from_header_text("id=a3fWa; Max-Age=1").unwrap();
        assert_eq!(cookie.max_age.unwrap(), Duration::seconds(1));
    }

    #[test]
    fn test_invalid_max_age() {
        let cookie = set_cookie_from_header_text("id=a3fWa; Max-Age=f");
        assert!(cookie.is_err());
    }

    #[test]
    fn test_cookie_expire_serialisation() {
        let mut cookie = SetCookie::new("id".to_string(), "a3fWa".to_string());

        cookie.set_expires(
            NaiveDate::from_ymd_opt(2015, 10, 21)
                .unwrap()
                .and_hms_opt(7, 28, 1)
                .unwrap(),
        );

        assert_eq!(
            cookie.to_string(),
            "id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:01 GMT"
        );
    }

    #[test]
    fn test_cookie_max_age_serialisation() {
        let mut cookie = SetCookie::new("id".to_string(), "a3fWa".to_string());

        cookie.set_max_age(Duration::minutes(1));

        assert_eq!(cookie.to_string(), "id=a3fWa; Max-Age=60");
    }

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

    #[test]
    fn test_cookie() {
        let mut cookie = SetCookie::new("a".to_string(), "b".to_string());
        cookie.set_max_age(Duration::seconds(1));
        let response = ResponseBuilder::new()
            .set_status_code(StatusCode::Continue)
            .add_cookie(cookie)
            .build();
        let expected = "HTTP/1.1 100 Continue\r\nSet-Cookie: a=b; Max-Age=1\r\n\r\n";
        assert_eq!(&response,expected)
    }

}
