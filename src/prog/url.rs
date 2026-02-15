use crate::api::*;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url<'a> {
    pub(super) scheme: Scheme,
    pub(super) authority: Option<Authority<'a>>,
    pub(super) host: Host,
    pub(super) port: Option<u16>,
    pub(super) path: Option<Vec<&'a str>>,
    pub(super) query: Option<QueryParams<'a>>,
    pub(super) fragment: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheme(pub(super) String);

pub type Authority<'a> = (&'a str, Option<&'a str>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Host {
    Host(String),
    IP([u8; 4]),
}

pub type QueryParam<'a> = (&'a str, &'a str);
pub type QueryParams<'a> = Vec<QueryParam<'a>>;

impl<'a> Url<'a> {
    pub(crate) fn parse(input: &str) -> Res<Url<'_>> {
        let input = input.trim();
        super::parse_url::parse_url(input)
    }
}

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

impl From<&str> for Scheme {
    fn from(i: &str) -> Self {
        Self(i.to_lowercase())
    }
}
