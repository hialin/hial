use crate::prog::url::*;

use super::ParseError;
use chumsky::prelude::*;

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

fn digits_between(min: usize, max: usize) -> impl Parser<char, String, Error = ParseError> + Clone {
    filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .at_least(min)
        .at_most(max)
        .collect::<String>()
}

fn alpha1() -> impl Parser<char, String, Error = ParseError> + Clone {
    filter(ascii_alpha)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

fn alphanumeric1() -> impl Parser<char, String, Error = ParseError> + Clone {
    filter(ascii_alnum)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

fn alphanumerichyphen1() -> impl Parser<char, String, Error = ParseError> + Clone {
    filter(ascii_alnum_or_hyphen)
        .repeated()
        .at_least(1)
        .collect::<String>()
}

fn scheme() -> impl Parser<char, Scheme, Error = ParseError> + Clone {
    url_code_points()
        .then_ignore(just("://"))
        .map(|s| Scheme::from(s.as_str()))
        .labelled("scheme")
}

fn authority<'a>() -> impl Parser<char, Authority<'a>, Error = ParseError> + Clone {
    alphanumeric1()
        .then(just(':').ignore_then(alphanumeric1()).or_not())
        .map(|(u, p)| (leak_str(u), p.map(leak_str)))
        .then_ignore(just('@'))
        .labelled("authority")
}

fn host<'a>() -> impl Parser<char, Host, Error = ParseError> + Clone {
    let dotted = alphanumerichyphen1()
        .then_ignore(just('.'))
        .repeated()
        .at_least(1)
        .then(alpha1())
        .map(|(mut head, tail)| {
            head.push(tail);
            Host::Host(head.join("."))
        });

    let single = alphanumerichyphen1().map(Host::Host);
    choice((dotted, single)).labelled("host")
}

fn ip_num() -> impl Parser<char, u8, Error = ParseError> + Clone {
    digits_between(1, 3)
        .try_map(|result: String, span| match result.parse::<u8>() {
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "invalid ip number")),
        })
        .labelled("ip number")
}

fn ip() -> impl Parser<char, Host, Error = ParseError> + Clone {
    ip_num()
        .then_ignore(just('.'))
        .repeated()
        .exactly(3)
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

fn ip_or_host<'a>() -> impl Parser<char, Host, Error = ParseError> + Clone {
    choice((ip(), host())).labelled("ip or host")
}

fn port() -> impl Parser<char, u16, Error = ParseError> + Clone {
    just(':')
        .ignore_then(digits_between(2, 4))
        .try_map(|res: String, span| match res.parse::<u16>() {
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "invalid port")),
        })
        .labelled("port")
}

fn url_path<'a>() -> impl Parser<char, Vec<&'a str>, Error = ParseError> + Clone {
    just('/')
        .ignore_then(url_code_points().separated_by(just('/')).allow_trailing())
        .map(|parts| parts.into_iter().map(leak_str).collect::<Vec<_>>())
        .labelled("url_path")
}

fn query_params<'a>() -> impl Parser<char, QueryParams<'a>, Error = ParseError> + Clone {
    let pair = url_code_points()
        .then_ignore(just('='))
        .then(url_code_points());
    just('?')
        .ignore_then(pair.separated_by(just('&')).at_least(1))
        .map(|parts| {
            parts
                .into_iter()
                .map(|(k, v)| (leak_str(k), leak_str(v)))
                .collect::<Vec<_>>()
        })
        .labelled("query params")
}

fn fragment() -> impl Parser<char, &'static str, Error = ParseError> + Clone {
    just('#')
        .ignore_then(url_code_points())
        .map(leak_str)
        .labelled("fragment")
}

pub(super) fn path_code_points() -> impl Parser<char, String, Error = ParseError> + Clone {
    let accept = |c: &char| {
        *c == '-' || *c == '_' || *c == '.' || *c == ':' || *c == '*' || c.is_ascii_alphanumeric()
    };
    filter(accept).repeated().at_least(1).collect::<String>()
}

pub(super) fn url_code_points() -> impl Parser<char, String, Error = ParseError> + Clone {
    let accept = |c: &char| *c == '-' || *c == '.' || c.is_ascii_alphanumeric();
    filter(accept).repeated().at_least(1).collect::<String>()
}

pub(super) fn url_parser<'a>() -> impl Parser<char, Url<'a>, Error = ParseError> + Clone {
    scheme()
        .then(authority().or_not())
        .then(ip_or_host())
        .then(port().or_not())
        .then(url_path().or_not())
        .then(query_params().or_not())
        .then(fragment().or_not())
        .map(
            |((((((scheme, authority), host), port), path), query), fragment)| Url {
                scheme,
                authority,
                host,
                port,
                path,
                query,
                fragment,
            },
        )
        .labelled("url")
}

///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_with<'a, O>(
        parser: impl Parser<char, O, Error = ParseError>,
        input: &'a str,
    ) -> Result<O, Vec<ParseError>> {
        parser.then_ignore(end()).parse(input)
    }

    #[test]
    fn test_scheme() {
        assert_eq!(scheme().parse("https://yay"), Ok(Scheme("https".into())));
        assert_eq!(scheme().parse("http://yay"), Ok(Scheme("http".into())));
        assert_eq!(scheme().parse("bla://yay"), Ok(Scheme("bla".into())));
        assert!(scheme().parse("bla:/yay").is_err());
    }

    #[test]
    fn test_authority() {
        assert_eq!(
            authority().parse("username:password@zupzup.org"),
            Ok(("username", Some("password")))
        );
        assert_eq!(
            authority().parse("username@zupzup.org"),
            Ok(("username", None))
        );
        assert!(authority().parse("zupzup.org").is_err());
        assert!(authority().parse(":zupzup.org").is_err());
        assert!(authority().parse("username:passwordzupzup.org").is_err());
        assert!(authority().parse("@zupzup.org").is_err());
    }

    #[test]
    fn test_host() {
        assert_eq!(
            host().parse("localhost:8080"),
            Ok(Host::Host("localhost".to_string()))
        );
        assert_eq!(
            host().parse("example.org:8080"),
            Ok(Host::Host("example.org".to_string()))
        );
        assert_eq!(
            host().parse("some-subsite.example.org:8080"),
            Ok(Host::Host("some-subsite.example.org".to_string()))
        );
        assert_eq!(
            host().parse("example.123"),
            Ok(Host::Host("example".to_string()))
        );
        assert!(host().parse("$$$.com").is_err());
        assert!(host().parse(".com").is_err());
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(
            ip().parse("192.168.0.1:8080"),
            Ok(Host::IP([192, 168, 0, 1]))
        );
        assert_eq!(ip().parse("0.0.0.0:8080"), Ok(Host::IP([0, 0, 0, 0])));
        assert!(ip().parse("1924.168.0.1:8080").is_err());
        assert!(ip().parse("192.168.0000.144:8080").is_err());
        assert_eq!(
            ip().parse("192.168.0.1444:8080"),
            Ok(Host::IP([192, 168, 0, 144]))
        );
        assert!(ip().parse("192.168.0:8080").is_err());
        assert!(ip().parse("999.168.0.0:8080").is_err());
    }

    #[test]
    fn test_url_path() {
        assert_eq!(url_path().parse("/a/b/c?d"), Ok(vec!["a", "b", "c"]));
        assert_eq!(url_path().parse("/a/b/c/?d"), Ok(vec!["a", "b", "c"]));
        assert_eq!(
            url_path().parse("/a/b-c-d/c/?d"),
            Ok(vec!["a", "b-c-d", "c"])
        );
        assert_eq!(url_path().parse("/a/1234/c/?d"), Ok(vec!["a", "1234", "c"]));
        assert_eq!(
            url_path().parse("/a/1234/c.txt?d"),
            Ok(vec!["a", "1234", "c.txt"])
        );
    }

    #[test]
    fn test_query_params() {
        assert_eq!(
            query_params().parse("?bla=5&blub=val#yay"),
            Ok(vec![("bla", "5"), ("blub", "val")])
        );

        assert_eq!(
            query_params().parse("?bla-blub=arr-arr#yay"),
            Ok(vec![("bla-blub", "arr-arr")])
        );
    }

    #[test]
    fn test_fragment() {
        assert_eq!(fragment().parse("#bla"), Ok("bla"));
        assert_eq!(fragment().parse("#bla-blub"), Ok("bla-blub"));
    }

    #[test]
    fn test_url() {
        assert_eq!(
            parse_with(url_parser(), "https://www.zupzup.org/about/"),
            Ok(Url {
                scheme: Scheme("https".into()),
                authority: None,
                host: Host::Host("www.zupzup.org".to_string()),
                port: None,
                path: Some(vec!["about"]),
                query: None,
                fragment: None
            })
        );

        assert_eq!(
            parse_with(url_parser(), "http://localhost"),
            Ok(Url {
                scheme: Scheme("http".into()),
                authority: None,
                host: Host::Host("localhost".to_string()),
                port: None,
                path: None,
                query: None,
                fragment: None
            })
        );

        assert_eq!(
            parse_with(
                url_parser(),
                "https://www.zupzup.org:443/about/?someVal=5#anchor"
            ),
            Ok(Url {
                scheme: Scheme("https".into()),
                authority: None,
                host: Host::Host("www.zupzup.org".to_string()),
                port: Some(443),
                path: Some(vec!["about"]),
                query: Some(vec![("someVal", "5")]),
                fragment: Some("anchor")
            })
        );

        assert_eq!(
            parse_with(url_parser(), "http://user:pw@127.0.0.1:8080"),
            Ok(Url {
                scheme: Scheme("http".into()),
                authority: Some(("user", Some("pw"))),
                host: Host::IP([127, 0, 0, 1]),
                port: Some(8080),
                path: None,
                query: None,
                fragment: None
            })
        );
    }
}
