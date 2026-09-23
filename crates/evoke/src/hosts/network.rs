//! One HTTP POST, one attempt, over a kept-alive agent with the bundled roots, through the proxy the environment
//! names: connect within the deadline, the policy's timeout for each step after connect; never retries. In: the
//! agent, url, bearer, body, deadline, timeout. Out: the status and body, or what went wrong in plain words and
//! whether a connection had been made.

use std::io;
use std::time::Duration;

use evoke_core::name::VarName;
use evoke_core::plan::Millis;
use evoke_core::{Diagnostic, Fault, Fix};
use ureq::http::Uri;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Error, Proxy, ProxyProtocol, Timeout};

use super::{Deadline, Environment};

/// The variables a proxy is read from, the lower-case name first, as Node reads them for the SDK.
const PROXY: [&str; 2] = ["https_proxy", "HTTPS_PROXY"];
const NO_PROXY: [&str; 2] = ["no_proxy", "NO_PROXY"];

/// A kept-alive agent: Mozilla's roots, never the platform's; the proxy `HTTPS_PROXY` names and the hosts
/// `NO_PROXY` exempts, the same two variables in both hosts; no redirect, since the endpoint never moves; a status
/// is a response, never an error.
pub struct Agent(ureq::Agent);

impl Agent {
    /// The agent, or the line that says the proxy variable holds no proxy address.
    pub fn new(environment: &Environment) -> Result<Self, Diagnostic> {
        let tls = TlsConfig::builder().root_certs(RootCerts::WebPki).build();
        Ok(Self(
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .max_redirects(0)
                .proxy(proxy(environment)?)
                .tls_config(tls)
                .build()
                .new_agent(),
        ))
    }
}

/// The proxy the environment names, or none: `http://[user:password@]host[:port]`, `https` allowed too; the
/// `NO_PROXY` list beside it. A value that is not such an address is refused rather than bypassed in silence.
fn proxy(environment: &Environment) -> Result<Option<Proxy>, Diagnostic> {
    let Some((var, value)) = PROXY
        .into_iter()
        .find_map(|var| environment.get(var).map(|value| (var, value)))
    else {
        return Ok(None);
    };
    let refused = || Diagnostic {
        reflex: None,
        at: None,
        message: format!("{var} is not an http or https proxy address"),
        fix: Fix::ExportKey {
            var: VarName::new(var).expect("the variable is a name"),
        },
    };
    let uri: Uri = value.parse().map_err(|_| refused())?;
    let protocol = match uri.scheme_str() {
        Some("http") => ProxyProtocol::Http,
        Some("https") => ProxyProtocol::Https,
        _ => return Err(refused()),
    };
    let (Some(authority), Some(host)) = (uri.authority(), uri.host()) else {
        return Err(refused());
    };
    let mut builder = Proxy::builder(protocol).host(host);
    if let Some(port) = uri.port_u16() {
        builder = builder.port(port);
    }
    if let Some((userinfo, _)) = authority.as_str().rsplit_once('@') {
        let (user, password) = userinfo.split_once(':').unwrap_or((userinfo, ""));
        builder = builder.username(user).password(password);
    }
    for entry in NO_PROXY
        .into_iter()
        .find_map(|var| environment.get(var))
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        builder = builder.no_proxy(entry);
    }
    builder.build().map(Some).map_err(|_| refused())
}

/// What the server answered.
pub struct Response {
    pub status: u16,
    pub body: String,
}

/// Why nothing was answered, in plain words, and whether a connection was made first: an attempt that never
/// connected is safe to repeat.
#[derive(Debug)]
pub struct Transport {
    pub connected: bool,
    pub message: String,
}

impl From<Transport> for Fault {
    fn from(transport: Transport) -> Self {
        Self::Transport {
            message: transport.message,
        }
    }
}

/// `POST url` with a bearer token and a JSON body.
pub fn post(
    agent: &Agent,
    url: &str,
    bearer: &str,
    body: &str,
    deadline: Deadline,
    timeout: Millis,
) -> Result<Response, Transport> {
    let remaining = deadline.remaining();
    if remaining.is_zero() {
        return Err(Transport {
            connected: false,
            message: "the deadline passed before connecting".to_owned(),
        });
    }
    let step = Some(Duration::from_millis(timeout.0));
    let sent = agent
        .0
        .post(url)
        .config()
        .timeout_global(Some(remaining))
        .timeout_send_request(step)
        .timeout_send_body(step)
        .timeout_recv_response(step)
        .timeout_recv_body(step)
        .build()
        .header("authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json")
        .send(body);
    let describe = |error: &Error| describe(error, agent.0.config().proxy(), url, timeout);
    match sent {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(|error| describe(&error))?;
            Ok(Response { status, body })
        }
        Err(error) => Err(describe(&error)),
    }
}

/// The failure in plain words: where the connection was going, the proxy when one carries it; the system's own
/// cause without its number; a step's timeout in seconds. And whether a connection was made before it.
fn describe(error: &Error, proxy: Option<&Proxy>, url: &str, timeout: Millis) -> Transport {
    let uri: Uri = url.parse().unwrap_or_default();
    let host = uri.host().unwrap_or(url);
    let via = match proxy {
        Some(proxy) if !proxy.is_no_proxy(&uri) => {
            format!("the proxy {}:{}", proxy.host(), proxy.port())
        }
        _ => host.to_owned(),
    };
    let (connected, message) = match error {
        Error::HostNotFound => (false, format!("could not resolve {via}")),
        Error::ConnectionFailed => (false, format!("could not connect to {via}")),
        Error::ConnectProxyFailed(status) => {
            (false, format!("{via} refused the connection: {status}"))
        }
        Error::Io(error) if unresolved(error) => (false, format!("could not resolve {via}")),
        Error::Timeout(Timeout::Resolve) => (false, format!("resolving {via} timed out")),
        Error::Timeout(Timeout::Connect) => (false, format!("connecting to {via} timed out")),
        Error::Timeout(Timeout::Global | Timeout::PerCall) => {
            (true, format!("the deadline passed waiting for {host}"))
        }
        Error::Timeout(_) => (
            true,
            format!("{host}: no answer within {}", seconds(timeout)),
        ),
        Error::Io(error) => match cause(error) {
            (false, cause) => (false, format!("could not connect to {via}: {cause}")),
            (true, cause) => (true, format!("{host}: {cause}")),
        },
        error => (true, format!("{host}: {error}")),
    };
    Transport { connected, message }
}

/// The resolver's failure, which std reports as an uncategorised io error with a fixed prefix.
fn unresolved(error: &io::Error) -> bool {
    error
        .to_string()
        .starts_with("failed to lookup address information")
}

/// Whether an io error came after a connection was made, and its cause as the system words it, without the
/// number: `connection refused`, `network is unreachable`.
fn cause(error: &io::Error) -> (bool, String) {
    let connected = !matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::NetworkUnreachable
            | io::ErrorKind::HostUnreachable
            | io::ErrorKind::AddrNotAvailable
            | io::ErrorKind::TimedOut
    );
    let text = error.to_string();
    let text = text.split(" (os error ").next().unwrap_or(&text);
    (connected, lowered(text))
}

/// The first letter lowered, unless the word is an acronym: the system says `Connection refused`.
fn lowered(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.as_str();
    if first.is_uppercase() && !rest.starts_with(char::is_uppercase) {
        first.to_lowercase().chain(rest.chars()).collect()
    } else {
        text.to_owned()
    }
}

/// Milliseconds as seconds: `1.5 s`, `30 s`.
fn seconds(ms: Millis) -> String {
    let (whole, rest) = (ms.0 / 1000, ms.0 % 1000);
    if rest == 0 {
        format!("{whole} s")
    } else {
        let fraction = format!("{rest:03}");
        format!("{whole}.{} s", fraction.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ureq::ProxyProtocol;

    const URL: &str = "https://api.typesafe.ai/v1/systemone";

    fn refused() -> Error {
        Error::Io(io::Error::from(io::ErrorKind::ConnectionRefused))
    }

    #[test]
    fn a_failure_names_the_host_and_its_cause_in_plain_words() {
        let said = |error: &Error| describe(error, None, URL, Millis(1500));
        let refused = said(&refused());
        assert!(!refused.connected);
        assert_eq!(
            refused.message,
            "could not connect to api.typesafe.ai: connection refused"
        );
        assert_eq!(
            said(&Error::HostNotFound).message,
            "could not resolve api.typesafe.ai"
        );
        assert_eq!(
            said(&Error::Timeout(Timeout::Connect)).message,
            "connecting to api.typesafe.ai timed out"
        );
        let slow = said(&Error::Timeout(Timeout::RecvResponse));
        assert!(slow.connected);
        assert_eq!(slow.message, "api.typesafe.ai: no answer within 1.5 s");
        assert_eq!(
            said(&Error::Timeout(Timeout::Global)).message,
            "the deadline passed waiting for api.typesafe.ai"
        );
        let reset = said(&Error::Io(io::Error::from(io::ErrorKind::ConnectionReset)));
        assert!(reset.connected);
        assert_eq!(reset.message, "api.typesafe.ai: connection reset");
    }

    #[test]
    fn the_proxy_is_named_when_it_carries_the_connection() {
        let proxy = Proxy::new("http://127.0.0.1:9").unwrap();
        let carried = describe(&refused(), Some(&proxy), URL, Millis(1500));
        assert_eq!(
            carried.message,
            "could not connect to the proxy 127.0.0.1:9: connection refused"
        );
        let denied = describe(
            &Error::ConnectProxyFailed("407".to_owned()),
            Some(&proxy),
            URL,
            Millis(1500),
        );
        assert!(!denied.connected);
        assert_eq!(
            denied.message,
            "the proxy 127.0.0.1:9 refused the connection: 407"
        );
        let offline = describe(
            &Error::Io(io::Error::other(
                "failed to lookup address information: Temporary failure in name resolution",
            )),
            None,
            URL,
            Millis(1500),
        );
        assert!(!offline.connected);
        assert_eq!(offline.message, "could not resolve api.typesafe.ai");
        let bypassed = Proxy::builder(ProxyProtocol::Http)
            .host("127.0.0.1")
            .port(9)
            .no_proxy("api.typesafe.ai")
            .build()
            .unwrap();
        let direct = describe(&refused(), Some(&bypassed), URL, Millis(1500));
        assert_eq!(
            direct.message,
            "could not connect to api.typesafe.ai: connection refused"
        );
    }

    #[test]
    fn the_proxy_comes_from_two_variables_and_a_bad_value_is_refused() {
        let env = |pairs: &[(&str, &str)]| {
            Environment(
                pairs
                    .iter()
                    .map(|(var, value)| ((*var).to_owned(), (*value).to_owned()))
                    .collect(),
            )
        };
        assert!(proxy(&env(&[])).unwrap().is_none());
        let named = proxy(&env(&[
            ("HTTPS_PROXY", "http://ana:s3cret@proxy.example.com:3128"),
            ("NO_PROXY", "localhost, .internal.example.com"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(named.host(), "proxy.example.com");
        assert_eq!(named.port(), 3128);
        assert_eq!(named.username(), Some("ana"));
        assert_eq!(named.password(), Some("s3cret"));
        assert!(named.is_no_proxy(&"https://api.internal.example.com/".parse().unwrap()));
        assert!(!named.is_no_proxy(&URL.parse().unwrap()));
        let lower = proxy(&env(&[
            ("https_proxy", "http://first:80"),
            ("HTTPS_PROXY", "http://second:80"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(lower.host(), "first");
        for bad in ["socks5://127.0.0.1:1080", "not a url", "127.0.0.1:8080"] {
            let problem = proxy(&env(&[("HTTPS_PROXY", bad)])).unwrap_err();
            assert_eq!(
                problem.message,
                "HTTPS_PROXY is not an http or https proxy address"
            );
            assert_eq!(
                problem.fix,
                Fix::ExportKey {
                    var: VarName::new("HTTPS_PROXY").unwrap()
                }
            );
        }
    }

    #[test]
    fn the_system_s_number_is_dropped_and_its_sentence_case_with_it() {
        assert_eq!(cause(&io::Error::other("Boom (os error 5)")).1, "boom");
        assert_eq!(cause(&io::Error::other("TLS gave up")).1, "TLS gave up");
        assert_eq!(seconds(Millis(1500)), "1.5 s");
        assert_eq!(seconds(Millis(30_000)), "30 s");
        assert_eq!(seconds(Millis(1_050)), "1.05 s");
    }
}
