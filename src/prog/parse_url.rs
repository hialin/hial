use crate::prog::url::*;

use super::{ParseError, convert_error};
use crate::api::*;
use chumsky::prelude::*;

pub fn parse_url(input: &str) -> Res<Url<'_>> {
    url_parser()
        .then_ignore(end())
        .parse(input)
        .into_result()
        .map_err(|err| usererr(convert_error(input, err)))
}

pub(super) fn url_parser<'a>() -> impl Parser<'a, &'a str, Url<'a>, extra::Err<ParseError<'a>>> + Clone {
    scheme()
        .then(choice((
            authority()
                .then(ip_or_host())
                .map(|(authority, host)| (Some(authority), host)),
            ip_or_host().map(|host| (None, host)),
        )))
        .then(port().or_not())
        .then(url_path().or_not())
        .then(query_params().or_not())
        .then(fragment().or_not())
        .map(
            |(((((scheme, (authority, host)), port), path), query), fragment)| Url {
                scheme,
                authority,
                host,
                port,
                path: path.and_then(|p| if p.is_empty() { None } else { Some(p) }),
                query,
                fragment,
            },
        )
        .labelled("url")
}

fn scheme<'src>() -> impl Parser<'src, &'src str, Scheme, extra::Err<ParseError<'src>>> + Clone {
    url_code_points()
        .then_ignore(just("://"))
        .map(|s| Scheme::from(s.as_str()))
        .labelled("scheme")
}

fn authority<'a>() -> impl Parser<'a, &'a str, Authority<'a>, extra::Err<ParseError<'a>>> + Clone {
    alphanumeric1()
        .then(just(':').ignore_then(alphanumeric1()).or_not())
        .map(|(u, p)| (leak_str(u), p.map(leak_str)))
        .then_ignore(just('@'))
        .labelled("authority")
}

fn ip_or_host<'src>() -> impl Parser<'src, &'src str, Host, extra::Err<ParseError<'src>>> + Clone {
    choice((ip(), host())).labelled("ip or host")
}

fn port<'src>() -> impl Parser<'src, &'src str, u16, extra::Err<ParseError<'src>>> + Clone {
    just(':')
        .ignore_then(digits_between(1, 5))
        .try_map(|res: String, span| match res.parse::<u16>() {
            Ok(n) => Ok(n),
            Err(_) => Err(chumsky::error::Rich::custom(span, "invalid port")),
        })
        .labelled("port")
}

fn ip<'src>() -> impl Parser<'src, &'src str, Host, extra::Err<ParseError<'src>>> + Clone {
    ip_num()
        .then_ignore(just('.'))
        .repeated()
        .exactly(3)
        .collect::<Vec<_>>()
        .then(ip_num())
        .map(|(head, tail)| {
            let mut result: [u8; 4] = [0, 0, 0, 0];
            head.into_iter()
                .enumerate()
                .for_each(|(i, v)| result[i] = v);
            result[3] = tail;
            Host::IP(result)
        })
        .labelled("ip")
}

fn host<'src>() -> impl Parser<'src, &'src str, Host, extra::Err<ParseError<'src>>> + Clone {
    let dotted = alphanumerichyphen1()
        .then_ignore(just('.'))
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(alpha1())
        .map(|(mut head, tail)| {
            head.push(tail);
            Host::Host(head.join("."))
        });

    let single = alphanumerichyphen1().map(Host::Host);
    choice((dotted, single)).labelled("host")
}

fn url_path<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, extra::Err<ParseError<'a>>> + Clone {
    just('/')
        .ignore_then(
            url_code_points()
                .separated_by(just('/'))
                .allow_trailing()
                .collect::<Vec<_>>()
                .or_not()
                .map(|parts| parts.unwrap_or_default()),
        )
        .map(|parts| parts.into_iter().map(leak_str).collect::<Vec<_>>())
        .labelled("url_path")
}

fn query_params<'a>() -> impl Parser<'a, &'a str, QueryParams<'a>, extra::Err<ParseError<'a>>> + Clone {
    let pair = url_code_points()
        .then_ignore(just('='))
        .then(url_code_points());
    just('?')
        .ignore_then(pair.separated_by(just('&')).at_least(1).collect::<Vec<_>>())
        .map(|parts| {
            parts
                .into_iter()
                .map(|(k, v)| (leak_str(k), leak_str(v)))
                .collect::<Vec<_>>()
        })
        .labelled("query params")
}

fn fragment<'src>() -> impl Parser<'src, &'src str, &'static str, extra::Err<ParseError<'src>>> + Clone {
    just('#')
        .ignore_then(url_code_points())
        .map(leak_str)
        .labelled("fragment")
}

fn ip_num<'src>() -> impl Parser<'src, &'src str, u8, extra::Err<ParseError<'src>>> + Clone {
    digits_between(1, 3)
        .try_map(|result: String, span| match result.parse::<u8>() {
            Ok(n) => Ok(n),
            Err(_) => Err(chumsky::error::Rich::custom(span, "invalid ip number")),
        })
        .labelled("ip number")
}

fn alphanumerichyphen1<'src>() -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    any().filter(ascii_alnum_or_hyphen)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

fn alphanumeric1<'src>() -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    any().filter(ascii_alnum)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

fn digits_between<'src>(
    min: usize,
    max: usize,
) -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    any().filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .at_least(min)
        .at_most(max)
        .collect::<String>()
}

fn alpha1<'src>() -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    any().filter(ascii_alpha)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

pub(super) fn path_code_points<'src>() -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    let accept = |c: &char| {
        *c == '-' || *c == '_' || *c == '.' || *c == ':' || *c == '*' || c.is_ascii_alphanumeric()
    };
    any().filter(accept).repeated().at_least(1).collect::<String>()
}

pub(super) fn url_code_points<'src>() -> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    let accept = |c: &char| *c == '-' || *c == '.' || c.is_ascii_alphanumeric();
    any().filter(accept).repeated().at_least(1).collect::<String>()
}

// TODO: this is not ok, remove this function and fix the problems
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn ascii_alpha(c: &char) -> bool {
    c.is_ascii_alphabetic()
}

fn ascii_alnum(c: &char) -> bool {
    c.is_ascii_alphanumeric()
}

fn ascii_alnum_or_hyphen(c: &char) -> bool {
    *c == '-' || c.is_ascii_alphanumeric()
}

///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_with<'src, O>(
        parser: impl Parser<'src, &'src str, O, extra::Err<ParseError<'src>>>,
        input: &'src str,
    ) -> Result<O, Vec<ParseError<'src>>> {
        parser.lazy().parse(input).into_result()
    }

    #[test]
    fn test_scheme() {
        assert_eq!(parse_with(scheme(), "https://yay"), Ok(Scheme("https".into())));
        assert_eq!(parse_with(scheme(), "http://yay"), Ok(Scheme("http".into())));
        assert_eq!(parse_with(scheme(), "bla://yay"), Ok(Scheme("bla".into())));
        assert!(parse_with(scheme(), "bla:/yay").is_err());
    }

    #[test]
    fn test_authority() {
        assert_eq!(
            parse_with(authority(), "username:password@zupzup.org"),
            Ok(("username", Some("password")))
        );
        assert_eq!(
            parse_with(authority(), "username@zupzup.org"),
            Ok(("username", None))
        );
        assert!(parse_with(authority(), "zupzup.org").is_err());
        assert!(parse_with(authority(), ":zupzup.org").is_err());
        assert!(parse_with(authority(), "username:passwordzupzup.org").is_err());
        assert!(parse_with(authority(), "@zupzup.org").is_err());
    }

    #[test]
    fn test_host() {
        assert_eq!(
            parse_with(host(), "localhost:8080"),
            Ok(Host::Host("localhost".to_string()))
        );
        assert_eq!(
            parse_with(host(), "example.org:8080"),
            Ok(Host::Host("example.org".to_string()))
        );
        assert_eq!(
            parse_with(host(), "some-subsite.example.org:8080"),
            Ok(Host::Host("some-subsite.example.org".to_string()))
        );
        assert_eq!(
            parse_with(host(), "example.123"),
            Ok(Host::Host("example".to_string()))
        );
        assert!(parse_with(host(), "$$$.com").is_err());
        assert!(parse_with(host(), ".com").is_err());
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(
            parse_with(ip(), "192.168.0.1:8080"),
            Ok(Host::IP([192, 168, 0, 1]))
        );
        assert_eq!(parse_with(ip(), "0.0.0.0:8080"), Ok(Host::IP([0, 0, 0, 0])));
        assert!(parse_with(ip(), "1924.168.0.1:8080").is_err());
        assert!(parse_with(ip(), "192.168.0000.144:8080").is_err());
        assert_eq!(
            parse_with(ip(), "192.168.0.1444:8080"),
            Ok(Host::IP([192, 168, 0, 144]))
        );
        assert!(parse_with(ip(), "192.168.0:8080").is_err());
        assert!(parse_with(ip(), "999.168.0.0:8080").is_err());
    }

    #[test]
    fn test_url_path() {
        assert_eq!(parse_with(url_path(), "/?d"), Ok(vec![]));
        assert_eq!(parse_with(url_path(), "/"), Ok(vec![]));
        assert_eq!(parse_with(url_path(), "/a/b/c?d"), Ok(vec!["a", "b", "c"]));
        assert_eq!(parse_with(url_path(), "/a/b/c/?d"), Ok(vec!["a", "b", "c"]));
        assert_eq!(
            parse_with(url_path(), "/a/b-c-d/c/?d"),
            Ok(vec!["a", "b-c-d", "c"])
        );
        assert_eq!(parse_with(url_path(), "/a/1234/c/?d"), Ok(vec!["a", "1234", "c"]));
        assert_eq!(
            parse_with(url_path(), "/a/1234/c.txt?d"),
            Ok(vec!["a", "1234", "c.txt"])
        );
    }

    #[test]
    fn test_query_params() {
        assert_eq!(
            parse_with(query_params(), "?bla=5&blub=val#yay"),
            Ok(vec![("bla", "5"), ("blub", "val")])
        );

        assert_eq!(
            parse_with(query_params(), "?bla-blub=arr-arr#yay"),
            Ok(vec![("bla-blub", "arr-arr")])
        );
    }

    #[test]
    fn test_fragment() {
        assert_eq!(parse_with(fragment(), "#bla"), Ok("bla"));
        assert_eq!(parse_with(fragment(), "#bla-blub"), Ok("bla-blub"));
    }

    #[test]
    fn test_url() {
        assert_eq!(
            parse_url("https://www.zupzup.org/about/").unwrap(),
            Url {
                scheme: Scheme("https".into()),
                authority: None,
                host: Host::Host("www.zupzup.org".to_string()),
                port: None,
                path: Some(vec!["about"]),
                query: None,
                fragment: None
            }
        );

        assert_eq!(
            parse_url("http://localhost").unwrap(),
            Url {
                scheme: Scheme("http".into()),
                authority: None,
                host: Host::Host("localhost".to_string()),
                port: None,
                path: None,
                query: None,
                fragment: None
            }
        );
        assert_eq!(
            parse_url("http://localhost:2/").unwrap(),
            Url {
                scheme: Scheme("http".into()),
                authority: None,
                host: Host::Host("localhost".to_string()),
                port: Some(2),
                path: None,
                query: None,
                fragment: None
            }
        );

        assert_eq!(
            parse_url("https://www.zupzup.org:443/about/?someVal=5#anchor").unwrap(),
            Url {
                scheme: Scheme("https".into()),
                authority: None,
                host: Host::Host("www.zupzup.org".to_string()),
                port: Some(443),
                path: Some(vec!["about"]),
                query: Some(vec![("someVal", "5")]),
                fragment: Some("anchor")
            }
        );

        assert_eq!(
            parse_url("http://user:pw@127.0.0.1:8080").unwrap(),
            Url {
                scheme: Scheme("http".into()),
                authority: Some(("user", Some("pw"))),
                host: Host::IP([127, 0, 0, 1]),
                port: Some(8080),
                path: None,
                query: None,
                fragment: None
            }
        );
    }
}
