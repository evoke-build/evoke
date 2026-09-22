//! One HTTP POST, one attempt, over a kept-alive agent with the bundled roots, through the proxy the environment
//! names: connect within the deadline, the policy's timeout for each step after connect; never retries. In: the
//! agent, url, bearer, body, deadline, timeout. Out: the status and body, or what went wrong in plain words and
//! whether a connection had been made.

use std::io;
use std::time::Duration;

use evoke_core::Fault;
use evoke_core::plan::Millis;
use ureq::http::Uri;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Error, Proxy, Timeout};

use super::Deadline;

/// A kept-alive agent: Mozilla's roots, never the platform's; the proxy `HTTPS_PROXY` and `NO_PROXY` name, as
/// ureq reads them; a status is a response, never an error.
pub struct Agent(ureq::Agent);

impl Agent {
    #[must_use]
    pub fn new() -> Self {
        let tls = TlsConfig::builder().root_certs(RootCerts::WebPki).build();
        Self(
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .tls_config(tls)
                .build()
                .new_agent(),
        )
    }
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
    fn the_system_s_number_is_dropped_and_its_sentence_case_with_it() {
        assert_eq!(cause(&io::Error::other("Boom (os error 5)")).1, "boom");
        assert_eq!(cause(&io::Error::other("TLS gave up")).1, "TLS gave up");
        assert_eq!(seconds(Millis(1500)), "1.5 s");
        assert_eq!(seconds(Millis(30_000)), "30 s");
        assert_eq!(seconds(Millis(1_050)), "1.05 s");
    }
}
