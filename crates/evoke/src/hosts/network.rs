//! One HTTP POST, one attempt, over a kept-alive agent with the bundled roots: connect within the deadline, the
//! policy's timeout for each step after connect; never retries. In: the agent, url, bearer, body, deadline, timeout.
//! Out: the status and body, or what went wrong and whether a connection had been made.

use std::io;
use std::time::Duration;

use evoke_core::Fault;
use evoke_core::plan::Millis;
use ureq::tls::{RootCerts, TlsConfig};

use super::Deadline;

/// A kept-alive agent: Mozilla's roots, never the platform's; a status is a response, never an error.
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

/// Why nothing was answered, and whether a connection was made first: an attempt that never connected is safe to
/// repeat.
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
    match sent {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(|error| Transport {
                    connected: true,
                    message: format!("reading the response: {error}"),
                })?;
            Ok(Response { status, body })
        }
        Err(error) => Err(Transport {
            connected: connected(&error),
            message: error.to_string(),
        }),
    }
}

/// Whether the failure came after a connection was made.
fn connected(error: &ureq::Error) -> bool {
    use ureq::{Error, Timeout};
    match error {
        Error::ConnectionFailed
        | Error::HostNotFound
        | Error::Timeout(Timeout::Resolve | Timeout::Connect) => false,
        Error::Io(error) => !matches!(
            error.kind(),
            io::ErrorKind::ConnectionRefused
                | io::ErrorKind::NetworkUnreachable
                | io::ErrorKind::HostUnreachable
                | io::ErrorKind::AddrNotAvailable
                | io::ErrorKind::TimedOut
        ),
        _ => true,
    }
}
