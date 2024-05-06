use std::{fmt::Display, str::FromStr};

use chrono::{Duration, Utc};
use regex::Replacer;

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
struct Cookie<T: Display> {
    name: String,
    value: T,
    domain: Option<String>,
    expires: Option<chrono::NaiveDateTime>,
    http_only: Option<bool>,
    max_age: Option<chrono::Duration>,
    partitioned: Option<bool>,
    path: Option<String>,
    same_site: Option<SameSite>,
    secure: Option<bool>,
}

impl<T: Display> Cookie<T> {
    fn new(name: String, value: T) -> Self {
        Cookie {
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

    fn set_max_age(&mut self, max_age: chrono::Duration) -> &Self {
        self.max_age = Some(max_age);
        self
    }
    fn set_expires(&mut self, expires: chrono::NaiveDateTime) -> &Self {
        self.expires = Some(expires);
        self
    }
}

impl<T: Display> Display for Cookie<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.value);
        if let Some(s) = &self.domain {
            write!(f, "; Domain={}", s);
        }
        if let Some(s) = &self.expires {
            let t = chrono::NaiveDateTime::format(&s, "%a, %d %b %Y %X GMT");
            write!(f, "; Expires={}", t);
        }
        if let Some(s) = &self.http_only {
            write!(f, "; HttpOnly");
        }
        if let Some(s) = &self.max_age {
            write!(f, "; Max-Age={}", s.num_seconds());
        }
        if let Some(s) = &self.partitioned {
            write!(f, "; Partitioned");
        }
        if let Some(s) = &self.path {
            write!(f, "; Path={}", s);
        }
        if let Some(s) = &self.same_site {
            write!(f, "; SameSite={}", s);
        }
        if let Some(s) = &self.secure {
            write!(f, "; Secure");
        }
        Ok(())
    }
}

fn from_header_text(header_text: &str) -> Result<Cookie<String>, String> {
    let mut header_info = header_text.split(";");
    let mut name_val = header_info.next().unwrap().split("=");
    let name = name_val.next().unwrap();
    let val = name_val.next().unwrap_or("");

    let mut cookie = Cookie::new(name.to_string(), val.to_string());

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
                        let t = v.parse().unwrap();
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate};

    use crate::cookies::{from_header_text, Cookie};

    #[test]
    fn test_simple_cookie_name() {
        let Cookie = from_header_text("id=a3fWa").unwrap();
        assert_eq!(Cookie.name, "id");
    }

    #[test]
    fn test_simple_cookie_value() {
        let Cookie = from_header_text("id=a3fWa").unwrap();
        assert_eq!(Cookie.value, "a3fWa");
    }

    #[test]
    fn test_cookie_expire_time() {
        let Cookie = from_header_text("id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:01 GMT").unwrap();
        assert_eq!(
            Cookie.expires.unwrap(),
            NaiveDate::from_ymd_opt(2015, 10, 21)
                .unwrap()
                .and_hms_opt(7, 28, 1)
                .unwrap()
        );
    }

    #[test]
    fn test_max_age() {
        let Cookie = from_header_text("id=a3fWa; Max-Age=1").unwrap();
        assert_eq!(Cookie.max_age.unwrap(), Duration::seconds(1));
    }

    #[test]
    fn test_cookie_expire_serialisation() {
        let mut cookie = Cookie::new("id".to_string(), "a3fWa".to_string());

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
        let mut cookie = Cookie::new("id".to_string(), "a3fWa".to_string());

        cookie.set_max_age(Duration::minutes(1));

        assert_eq!(cookie.to_string(), "id=a3fWa; Max-Age=60");
    }
}
