use std::collections::HashMap;

use hyper::Method;

use crate::handler::BoxHandler;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Static(String),
    /// `:name`, matches exactly one segment.
    Param(String),
    /// `*name` (last segment only), matches the rest of the path.
    Wildcard(String),
}

struct Route {
    method: Method,
    segments: Vec<Segment>,
    handler: BoxHandler,
}

pub(crate) enum Match<'a> {
    Found(&'a BoxHandler, HashMap<String, String>),
    MethodNotAllowed,
    NotFound,
}

#[derive(Default)]
pub(crate) struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub(crate) fn add(&mut self, method: Method, path: &str, handler: BoxHandler) {
        let segments = parse(path);
        if let Some(pos) = segments
            .iter()
            .position(|s| matches!(s, Segment::Wildcard(_)))
        {
            assert!(
                pos == segments.len() - 1,
                "hammock: wildcard must be the last segment in route `{path}`"
            );
        }
        self.routes.push(Route {
            method,
            segments,
            handler,
        });
    }

    /// Finds a handler. Routes are tried in registration order, but a route made only
    /// of static segments always wins over one with params or wildcards.
    pub(crate) fn find(&self, method: &Method, path: &str) -> Match<'_> {
        let parts: Vec<&str> = split(path).collect();
        let mut path_matched = false;
        let mut best: Option<(&Route, HashMap<String, String>)> = None;

        for route in &self.routes {
            let Some(params) = match_segments(&route.segments, &parts) else {
                continue;
            };
            path_matched = true;
            if route.method != *method {
                continue;
            }
            let is_static = params.is_empty()
                && route
                    .segments
                    .iter()
                    .all(|s| matches!(s, Segment::Static(_)));
            if is_static {
                return Match::Found(&route.handler, params);
            }
            if best.is_none() {
                best = Some((route, params));
            }
        }

        match best {
            Some((route, params)) => Match::Found(&route.handler, params),
            None if path_matched => Match::MethodNotAllowed,
            None => Match::NotFound,
        }
    }
}

fn split(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|s| !s.is_empty())
}

fn parse(path: &str) -> Vec<Segment> {
    split(path)
        .map(|s| {
            if let Some(name) = s.strip_prefix(':') {
                Segment::Param(name.to_owned())
            } else if let Some(name) = s.strip_prefix('*') {
                Segment::Wildcard(name.to_owned())
            } else {
                Segment::Static(s.to_owned())
            }
        })
        .collect()
}

fn match_segments(segments: &[Segment], parts: &[&str]) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    for (i, seg) in segments.iter().enumerate() {
        match seg {
            Segment::Wildcard(name) => {
                params.insert(name.clone(), parts.get(i..).unwrap_or(&[]).join("/"));
                return Some(params);
            }
            Segment::Static(s) => {
                if parts.get(i) != Some(&s.as_str()) {
                    return None;
                }
            }
            Segment::Param(name) => {
                params.insert(name.clone(), (*parts.get(i)?).to_owned());
            }
        }
    }
    (segments.len() == parts.len()).then_some(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(route: &str, path: &str) -> Option<HashMap<String, String>> {
        match_segments(&parse(route), &split(path).collect::<Vec<_>>())
    }

    #[test]
    fn matches_static_and_params() {
        assert!(m("/", "/").is_some());
        assert!(m("/users", "/users/").is_some());
        assert!(m("/users", "/users/1").is_none());
        assert_eq!(m("/users/:id", "/users/42").unwrap()["id"], "42");
        assert!(m("/users/:id", "/users").is_none());
    }

    #[test]
    fn matches_wildcard() {
        assert_eq!(m("/files/*path", "/files/a/b/c").unwrap()["path"], "a/b/c");
        assert_eq!(m("/files/*path", "/files").unwrap()["path"], "");
    }
}
