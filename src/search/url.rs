use super::parse::NomRes;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{alpha1, alphanumeric1, one_of},
    combinator::opt,
    error::{context, ErrorKind, VerboseError},
    multi::{count, many0, many1, many_m_n},
    sequence::{preceded, separated_pair, terminated, tuple},
    AsChar, Err as NomErr, InputTakeAtPosition,
};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url<'a> {
    scheme: Scheme,
    authority: Option<Authority<'a>>,
    host: Host,
    port: Option<u16>,
    path: Option<Vec<&'a str>>,
    query: Option<QueryParams<'a>>,
    fragment: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheme(String);

pub type Authority<'a> = (&'a str, Option<&'a str>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Host {
    Host(String),
    IP([u8; 4]),
}

pub type QueryParam<'a> = (&'a str, &'a str);
pub type QueryParams<'a> = Vec<QueryParam<'a>>;

impl<'a> Display for Url<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.scheme.0)?;
        f.write_str("://")?;
        if let Some(auth) = self.authority {
            f.write_str(auth.0)?;
            if let Some(auth2) = auth.1 {
                f.write_str(auth2)?;
            }
        }
        match &self.host {
            Host::Host(s) => f.write_str(s)?,
            Host::IP(ip) => f.write_fmt(format_args!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]))?,
        }
        if let Some(port) = self.port {
            f.write_fmt(format_args!(":{}", port))?;
        }
        if let Some(path) = &self.path {
            for p in path {
                f.write_str("/")?;
                f.write_str(p)?;
            }
        }
        if let Some(query) = &self.query {
            f.write_str("?")?;
            for (i, q) in query.iter().enumerate() {
                f.write_str(q.0)?;
                f.write_str("=")?;
                f.write_str(q.1)?;
                if i + 1 < query.len() {
                    f.write_str("&")?;
                }
            }
        }
        if let Some(fragment) = &self.fragment {
            f.write_str(fragment)?;
        }
        Ok(())
    }
}

pub fn url(input: &str) -> NomRes<&str, Url> {
    context(
        "url",
        tuple((
            scheme,
            opt(authority),
            ip_or_host,
            opt(port),
            opt(url_path),
            opt(query_params),
            opt(fragment),
        )),
    )(input)
    .map(|(next_input, res)| {
        let (scheme, authority, host, port, path, query, fragment) = res;
        (
            next_input,
            Url {
                scheme,
                authority,
                host,
                port,
                path,
                query,
                fragment,
            },
        )
    })
}

impl From<&str> for Scheme {
    fn from(i: &str) -> Self {
        Self(i.to_lowercase())
    }
}

fn scheme(input: &str) -> NomRes<&str, Scheme> {
    context(
        "scheme",
        terminated(url_code_points, tag("://")),
        // alt((tag_no_case("HTTP://"), tag_no_case("HTTPS://"))),
    )(input)
    .map(|(next_input, res)| (next_input, res.into()))
}

fn authority(input: &str) -> NomRes<&str, (&str, Option<&str>)> {
    context(
        "authority",
        terminated(
            separated_pair(alphanumeric1, opt(tag(":")), opt(alphanumeric1)),
            tag("@"),
        ),
    )(input)
}

fn ip_or_host(input: &str) -> NomRes<&str, Host> {
    context("ip or host", alt((ip, host)))(input)
}

fn host(input: &str) -> NomRes<&str, Host> {
    context(
        "host",
        alt((
            tuple((many1(terminated(alphanumerichyphen1, tag("."))), alpha1)),
            tuple((many_m_n(1, 1, alphanumerichyphen1), take(0_usize))),
        )),
    )(input)
    .map(|(next_input, mut res)| {
        if !res.1.is_empty() {
            res.0.push(res.1);
        }
        (next_input, Host::Host(res.0.join(".")))
    })
}

fn ip(input: &str) -> NomRes<&str, Host> {
    context(
        "ip",
        tuple((count(terminated(ip_num, tag(".")), 3), ip_num)),
    )(input)
    .map(|(next_input, res)| {
        let mut result: [u8; 4] = [0, 0, 0, 0];
        res.0
            .into_iter()
            .enumerate()
            .for_each(|(i, v)| result[i] = v);
        result[3] = res.1;
        (next_input, Host::IP(result))
    })
}

fn ip_num(input: &str) -> NomRes<&str, u8> {
    context("ip number", n_to_m_digits(1, 3))(input).and_then(|(next_input, result)| {
        match result.parse::<u8>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(NomErr::Error(VerboseError { errors: vec![] })),
        }
    })
}

fn port(input: &str) -> NomRes<&str, u16> {
    context("port", preceded(tag(":"), n_to_m_digits(2, 4)))(input).and_then(|(next_input, res)| {
        match res.parse::<u16>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(NomErr::Error(VerboseError { errors: vec![] })),
        }
    })
}

fn url_path(input: &str) -> NomRes<&str, Vec<&str>> {
    context(
        "url_path",
        tuple((
            tag("/"),
            many0(terminated(url_code_points, tag("/"))),
            opt(url_code_points),
        )),
    )(input)
    .map(|(next_input, res)| {
        let mut url_path: Vec<&str> = res.1.iter().map(|p| p.to_owned()).collect();
        if let Some(last) = res.2 {
            url_path.push(last);
        }
        (next_input, url_path)
    })
}

fn query_params(input: &str) -> NomRes<&str, QueryParams> {
    context(
        "query params",
        tuple((
            tag("?"),
            url_code_points,
            tag("="),
            url_code_points,
            many0(tuple((
                tag("&"),
                url_code_points,
                tag("="),
                url_code_points,
            ))),
        )),
    )(input)
    .map(|(next_input, res)| {
        let mut qps = Vec::new();
        qps.push((res.1, res.3));
        for qp in res.4 {
            qps.push((qp.1, qp.3));
        }
        (next_input, qps)
    })
}

fn fragment(input: &str) -> NomRes<&str, &str> {
    context("fragment", tuple((tag("#"), url_code_points)))(input)
        .map(|(next_input, res)| (next_input, res.1))
}
fn alphanumerichyphen1<T>(i: T) -> NomRes<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-' || char_item.is_alphanum())
        },
        ErrorKind::AlphaNumeric,
    )
}

pub fn path_code_points<T>(i: T) -> NomRes<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-'
                || char_item == '_'
                || char_item == '.'
                || char_item == ':'
                || char_item == '*'
                || char_item.is_alphanum())
        },
        ErrorKind::AlphaNumeric,
    )
}

pub fn url_code_points<T>(i: T) -> NomRes<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-' || char_item.is_alphanum() || char_item == '.')
            // ... actual ascii code points and url encoding...: https://infra.spec.whatwg.org/#ascii-code-point
        },
        ErrorKind::AlphaNumeric,
    )
}

pub fn identifier_code_points<T>(i: T) -> NomRes<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item.is_alphanum())
        },
        ErrorKind::AlphaNumeric,
    )
}

fn n_to_m_digits<'a>(n: usize, m: usize) -> impl FnMut(&'a str) -> NomRes<&str, String> {
    move |input| {
        many_m_n(n, m, one_of("0123456789"))(input)
            .map(|(next_input, result)| (next_input, result.into_iter().collect()))
    }
}

///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{
        error::{ErrorKind, VerboseError, VerboseErrorKind},
        Err as NomErr,
    };

    #[test]
    fn test_scheme() {
        assert_eq!(scheme("https://yay"), Ok(("yay", Scheme("https".into()))));
        assert_eq!(scheme("http://yay"), Ok(("yay", Scheme("http".into()))));
        assert_eq!(scheme("bla://yay"), Ok(("yay", Scheme("bla".into()))));
        assert_eq!(
            scheme("bla:/yay"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (":/yay", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("bla:/yay", VerboseErrorKind::Context("scheme")),
                ]
            }))
        );
    }

    #[test]
    fn test_authority() {
        assert_eq!(
            authority("username:password@zupzup.org"),
            Ok(("zupzup.org", ("username", Some("password"))))
        );
        assert_eq!(
            authority("username@zupzup.org"),
            Ok(("zupzup.org", ("username", None)))
        );
        assert_eq!(
            authority("zupzup.org"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (".org", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("zupzup.org", VerboseErrorKind::Context("authority")),
                ]
            }))
        );
        assert_eq!(
            authority(":zupzup.org"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (
                        ":zupzup.org",
                        VerboseErrorKind::Nom(ErrorKind::AlphaNumeric)
                    ),
                    (":zupzup.org", VerboseErrorKind::Context("authority")),
                ]
            }))
        );
        assert_eq!(
            authority("username:passwordzupzup.org"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (".org", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    (
                        "username:passwordzupzup.org",
                        VerboseErrorKind::Context("authority")
                    ),
                ]
            }))
        );
        assert_eq!(
            authority("@zupzup.org"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (
                        "@zupzup.org",
                        VerboseErrorKind::Nom(ErrorKind::AlphaNumeric)
                    ),
                    ("@zupzup.org", VerboseErrorKind::Context("authority")),
                ]
            }))
        )
    }

    #[test]
    fn test_host() {
        assert_eq!(
            host("localhost:8080"),
            Ok((":8080", Host::Host("localhost".to_string())))
        );
        assert_eq!(
            host("example.org:8080"),
            Ok((":8080", Host::Host("example.org".to_string())))
        );
        assert_eq!(
            host("some-subsite.example.org:8080"),
            Ok((":8080", Host::Host("some-subsite.example.org".to_string())))
        );
        assert_eq!(
            host("example.123"),
            Ok((".123", Host::Host("example".to_string())))
        );
        assert_eq!(
            host("$$$.com"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("$$$.com", VerboseErrorKind::Nom(ErrorKind::AlphaNumeric)),
                    ("$$$.com", VerboseErrorKind::Nom(ErrorKind::ManyMN)),
                    ("$$$.com", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    ("$$$.com", VerboseErrorKind::Context("host")),
                ]
            }))
        );
        assert_eq!(
            host(".com"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (".com", VerboseErrorKind::Nom(ErrorKind::AlphaNumeric)),
                    (".com", VerboseErrorKind::Nom(ErrorKind::ManyMN)),
                    (".com", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    (".com", VerboseErrorKind::Context("host")),
                ]
            }))
        );
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(
            ip("192.168.0.1:8080"),
            Ok((":8080", Host::IP([192, 168, 0, 1])))
        );
        assert_eq!(ip("0.0.0.0:8080"), Ok((":8080", Host::IP([0, 0, 0, 0]))));
        assert_eq!(
            ip("1924.168.0.1:8080"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("4.168.0.1:8080", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("1924.168.0.1:8080", VerboseErrorKind::Nom(ErrorKind::Count)),
                    ("1924.168.0.1:8080", VerboseErrorKind::Context("ip")),
                ]
            }))
        );
        assert_eq!(
            ip("192.168.0000.144:8080"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("0.144:8080", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    (
                        "192.168.0000.144:8080",
                        VerboseErrorKind::Nom(ErrorKind::Count)
                    ),
                    ("192.168.0000.144:8080", VerboseErrorKind::Context("ip")),
                ]
            }))
        );
        assert_eq!(
            ip("192.168.0.1444:8080"),
            Ok(("4:8080", Host::IP([192, 168, 0, 144])))
        );
        assert_eq!(
            ip("192.168.0:8080"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (":8080", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("192.168.0:8080", VerboseErrorKind::Nom(ErrorKind::Count)),
                    ("192.168.0:8080", VerboseErrorKind::Context("ip")),
                ]
            }))
        );
        assert_eq!(
            ip("999.168.0.0:8080"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("999.168.0.0:8080", VerboseErrorKind::Nom(ErrorKind::Count)),
                    ("999.168.0.0:8080", VerboseErrorKind::Context("ip")),
                ]
            }))
        );
    }

    #[test]
    fn test_url_path() {
        assert_eq!(url_path("/a/b/c?d"), Ok(("?d", vec!["a", "b", "c"])));
        assert_eq!(url_path("/a/b/c/?d"), Ok(("?d", vec!["a", "b", "c"])));
        assert_eq!(
            url_path("/a/b-c-d/c/?d"),
            Ok(("?d", vec!["a", "b-c-d", "c"]))
        );
        assert_eq!(url_path("/a/1234/c/?d"), Ok(("?d", vec!["a", "1234", "c"])));
        assert_eq!(
            url_path("/a/1234/c.txt?d"),
            Ok(("?d", vec!["a", "1234", "c.txt"]))
        );
    }

    #[test]
    fn test_query_params() {
        assert_eq!(
            query_params("?bla=5&blub=val#yay"),
            Ok(("#yay", vec![("bla", "5"), ("blub", "val")]))
        );

        assert_eq!(
            query_params("?bla-blub=arr-arr#yay"),
            Ok(("#yay", vec![("bla-blub", "arr-arr"),]))
        );
    }

    #[test]
    fn test_fragment() {
        assert_eq!(fragment("#bla"), Ok(("", "bla")));
        assert_eq!(fragment("#bla-blub"), Ok(("", "bla-blub")));
    }

    #[test]
    fn test_url() {
        assert_eq!(
            url("https://www.zupzup.org/about/"),
            Ok((
                "",
                Url {
                    scheme: Scheme("https".into()),
                    authority: None,
                    host: Host::Host("www.zupzup.org".to_string()),
                    port: None,
                    path: Some(vec!["about"]),
                    query: None,
                    fragment: None
                }
            ))
        );

        assert_eq!(
            url("http://localhost"),
            Ok((
                "",
                Url {
                    scheme: Scheme("http".into()),
                    authority: None,
                    host: Host::Host("localhost".to_string()),
                    port: None,
                    path: None,
                    query: None,
                    fragment: None
                }
            ))
        );

        assert_eq!(
            url("https://www.zupzup.org:443/about/?someVal=5#anchor"),
            Ok((
                "",
                Url {
                    scheme: Scheme("https".into()),
                    authority: None,
                    host: Host::Host("www.zupzup.org".to_string()),
                    port: Some(443),
                    path: Some(vec!["about"]),
                    query: Some(vec![("someVal", "5")]),
                    fragment: Some("anchor")
                }
            ))
        );

        assert_eq!(
            url("http://user:pw@127.0.0.1:8080"),
            Ok((
                "",
                Url {
                    scheme: Scheme("http".into()),
                    authority: Some(("user", Some("pw"))),
                    host: Host::IP([127, 0, 0, 1]),
                    port: Some(8080),
                    path: None,
                    query: None,
                    fragment: None
                }
            ))
        );
    }
}
