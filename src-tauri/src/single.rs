//! One client per user: the first binds a local port; a second launch (app
//! menu, login) asks it to show its window and exits.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

/// `SMASHCRAFT_CLIENT_PORT` overrides it for tests.
pub const CLIENT_PORT: u16 = 47632;
const SHOW: &[u8] = b"show\n";

pub fn client_port() -> u16 {
    std::env::var("SMASHCRAFT_CLIENT_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(CLIENT_PORT)
}

pub enum Claim {
    /// This is the only client; serve [`listen`] on it.
    First(TcpListener),
    /// Another client is running and was asked to show itself.
    Shown,
}

pub fn claim(port: u16) -> Claim {
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
        return Claim::First(listener);
    }
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(mut stream) => {
            let _ = stream.write_all(SHOW);
            Claim::Shown
        }
        // Something else holds the port: run anyway rather than refuse to start.
        Err(_) => Claim::First(TcpListener::bind(("127.0.0.1", 0)).expect("a local port")),
    }
}

/// Calls `show` for every later launch. Blocks; run it on a thread.
pub fn listen(listener: TcpListener, show: impl Fn()) {
    for stream in listener.incoming().map_while(Result::ok) {
        let mut stream = stream;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        let mut buf = [0u8; 5];
        if stream.read_exact(&mut buf).is_ok() && buf == SHOW[..5] {
            show();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn a_second_launch_shows_the_first_and_does_not_run() {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let Claim::First(listener) = claim(port) else { panic!("first launch must run") };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || listen(listener, move || tx.send(()).unwrap()));
        assert!(matches!(claim(port), Claim::Shown));
        rx.recv_timeout(Duration::from_secs(5)).expect("first client asked to show");
    }
}
