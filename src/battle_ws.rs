//! Optional WebSocket adapter for the `battle-tet.bot/v1` protocol.
//!
//! The game/search code deliberately has no dependency on this transport.  A
//! server sends an authoritative `decision.request`; this module converts the
//! visible state into the existing Guideline search state and sends exactly
//! one `decision.response`.  The adapter never simulates the referee and never
//! accepts a board update from the bot as authoritative.

use crate::{
    game::{Game, PIECES},
    search::{Budget, Searcher},
};
use intetrigence_engine::data::{Board, Piece, Placement, Rotation};
use rand::RngCore;
use serde_json::{json, Value};
#[cfg(feature = "battle-wss")]
use std::sync::Arc;
use std::{
    collections::VecDeque,
    fmt,
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};

const PROTOCOL_VERSION: &str = "battle-tet.bot/v1";
const MAX_HTTP_HEADERS: usize = 64 * 1024;
const MAX_FRAME_PAYLOAD: u64 = 4 * 1024 * 1024;
const HANDSHAKE_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Run the bot until the server closes the WebSocket.
pub fn run(url: &str, budget: Budget, weights: Option<&Value>) -> Result<(), String> {
    let mut socket = WsClient::connect(url)?;
    let mut searcher = Searcher::new(true, weights);

    loop {
        let Some(message) = socket.receive_text()? else {
            return Ok(());
        };
        let value: Value = serde_json::from_str(&message)
            .map_err(|error| format!("server sent invalid JSON: {error}"))?;
        validate_server_metadata(&value)?;
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "decision.request" => {
                let response = decision_response(&value, &mut searcher, budget)?;
                socket.send_text(&serde_json::to_string(&response).map_err(|e| e.to_string())?)?;
            }
            // The profile leaves room for connection, ruleset, and match
            // notifications. They are informational; the request state is
            // the only state used to make a move.
            "connection.ready" | "match.start" | "match.state" | "ruleset" | "pong"
            | "decision.cancel" => {}
            "error" => {
                return Err(format!("server rejected bot: {value}"));
            }
            _ => {
                // Unknown extension messages must not desynchronize the JSON
                // stream. The server remains authoritative for all actions.
            }
        }
    }
}

fn validate_server_metadata(message: &Value) -> Result<(), String> {
    let Some(version) = message.get("protocolVersion").and_then(Value::as_str) else {
        return Ok(());
    };
    // The heading in early battle.tet drafts used a dot while the fixed
    // protocol identifier uses a hyphen. Accept both spellings during the
    // rollout, while refusing a different protocol silently would be unsafe.
    if version != PROTOCOL_VERSION && version != "battle.tet.bot/v1" {
        return Err(format!(
            "unsupported battle.tet protocol version {version:?}"
        ));
    }
    Ok(())
}

fn decision_response(
    request: &Value,
    searcher: &mut Searcher,
    budget: Budget,
) -> Result<Value, String> {
    let request_id = request
        .get("requestId")
        .ok_or("decision.request.requestId is required")?;
    if request_id.is_null() {
        return Err("decision.request.requestId must not be null".into());
    }
    let state = request
        .get("state")
        .ok_or("decision.request.state is required")?;
    let (game, active, _next, _hold, can_hold) = game_from_state(state)?;

    // The server's deadline is authoritative. Leave a small margin for JSON
    // serialization and a network write. `--ms` remains the user's upper
    // bound, so a local run can use a smaller budget when desired.
    let deadline_ms = request
        .get("deadlineMs")
        .and_then(Value::as_u64)
        .unwrap_or(500);
    let margin_ms = if deadline_ms > 20 { 10 } else { 1 };
    let available_ms = deadline_ms.saturating_sub(margin_ms).max(1);
    let search_ms = if budget.milliseconds == 0 {
        available_ms
    } else {
        budget.milliseconds.min(available_ms)
    };
    let search_budget = Budget {
        milliseconds: search_ms,
        // An iteration budget supplied to the local referee is not allowed to
        // override a remote server deadline.
        iterations: 0,
    };

    let started = Instant::now();
    let (candidate, stats) = searcher.choose_transport(&game, search_budget);
    let chosen = candidate
        .filter(|mv| can_hold || mv.location.piece == active)
        .or_else(|| active_fallback(&game, active));
    let Some(mv) = chosen else {
        return Err(format!("no legal placement for request {}", request_id));
    };

    let use_hold = mv.location.piece != active;
    if use_hold && !can_hold {
        return Err("search selected HOLD while state.canHold is false".into());
    }
    let action = json!({
        "kind": "placement",
        // Engine locations use the rotation pivot as x=4 at spawn. The
        // protocol exposes the left edge of the 4x4 local box, x=3 at spawn.
        "x": i64::from(mv.location.x) - 1,
        "rotation": protocol_rotation(mv.location.rotation),
        "useHold": use_hold,
    });
    let elapsed_ms = started.elapsed().as_millis();
    if elapsed_ms >= u128::from(deadline_ms) {
        eprintln!(
            "battle-tet: decision {} exceeded deadline ({}ms >= {}ms)",
            request_id, elapsed_ms, deadline_ms
        );
    }
    eprintln!(
        "battle-tet: request {} -> x={} r={} hold={} ({} nodes, {}us)",
        request_id, action["x"], action["rotation"], use_hold, stats.nodes, stats.elapsed_us
    );
    Ok(json!({
        "type": "decision.response",
        "requestId": request_id,
        "action": action,
    }))
}

fn active_fallback(game: &Game, active: Piece) -> Option<Placement> {
    intetrigence_engine::movegen::find_moves(&game.board, active)
        .into_iter()
        .map(|(placement, _)| placement)
        .find(|placement| game.legal(*placement))
}

fn game_from_state(
    state: &Value,
) -> Result<(Game, Piece, Vec<Piece>, Option<Piece>, bool), String> {
    let active = parse_piece(
        state
            .get("active")
            .and_then(Value::as_str)
            .ok_or("state.active is required")?,
    )?;
    let hold = match state.get("hold") {
        None | Some(Value::Null) => None,
        Some(value) => Some(parse_piece(
            value
                .as_str()
                .ok_or("state.hold must be a piece letter or null")?,
        )?),
    };
    let next_values = state
        .get("next")
        .and_then(Value::as_array)
        .ok_or("state.next is required")?;
    if next_values.len() != 5 {
        return Err(format!(
            "state.next must contain exactly 5 pieces, got {}",
            next_values.len()
        ));
    }
    let next = next_values
        .iter()
        .map(|value| {
            parse_piece(
                value
                    .as_str()
                    .ok_or("state.next entries must be piece letters")?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let can_hold = state
        .get("canHold")
        .and_then(Value::as_bool)
        .ok_or("state.canHold is required")?;

    let board = parse_board(state)?;
    let incoming = state
        .get("incomingGarbage")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if incoming > 10_000 {
        return Err("state.incomingGarbage is unreasonably large".into());
    }

    // The protocol intentionally does not disclose the randomizer bag. The
    // search therefore uses only active + NEXT and disables speculative future
    // pieces. This is a transport limitation, not a dependency on the server
    // implementation.
    let mut game = Game::new(0);
    game.board = board;
    game.hold = hold;
    game.queue = std::iter::once(active)
        .chain(next.iter().copied())
        .collect::<VecDeque<_>>();
    game.pending = std::iter::repeat_n(255, incoming as usize).collect();
    game.bag = PIECES.to_vec();
    game.speculate = false;
    game.score = state.get("score").and_then(Value::as_u64).unwrap_or(0);
    game.lines = state.get("lines").and_then(Value::as_u64).unwrap_or(0) as u32;
    game.dead = false;
    Ok((game, active, next, hold, can_hold))
}

fn parse_board(state: &Value) -> Result<Board, String> {
    let rows = state
        .get("boardRows")
        .and_then(Value::as_array)
        .ok_or("state.boardRows is required")?;
    if rows.is_empty() || rows.len() > 40 {
        return Err("state.boardRows must contain between 1 and 40 rows".into());
    }
    let mut board = Board::default();
    for (y, row) in rows.iter().enumerate() {
        let row = row
            .as_str()
            .ok_or("state.boardRows entries must be 10-character strings")?;
        if row.chars().count() != 10 {
            return Err("state.boardRows entries must be exactly 10 characters".into());
        }
        for (x, cell) in row.chars().enumerate() {
            match cell {
                '0' => {}
                '1' => board.cols[x] |= 1u64 << y,
                _ => return Err("state.boardRows may contain only '0' and '1'".into()),
            }
        }
    }
    Ok(board)
}

fn parse_piece(value: &str) -> Result<Piece, String> {
    match value.to_ascii_uppercase().as_str() {
        "I" => Ok(Piece::I),
        "O" => Ok(Piece::O),
        "T" => Ok(Piece::T),
        "S" => Ok(Piece::S),
        "Z" => Ok(Piece::Z),
        "J" => Ok(Piece::J),
        "L" => Ok(Piece::L),
        _ => Err(format!("unknown piece {value:?}")),
    }
}

fn protocol_rotation(rotation: Rotation) -> u8 {
    match rotation {
        Rotation::North => 0,
        Rotation::East => 1,
        Rotation::South => 2,
        Rotation::West => 3,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Endpoint {
    secure: bool,
    host: String,
    port: u16,
    authority: String,
    path: String,
}

fn parse_endpoint(url: &str) -> Result<Endpoint, String> {
    let (secure, rest) = if let Some(rest) = url.strip_prefix("ws://") {
        (false, rest)
    } else if let Some(rest) = url.strip_prefix("wss://") {
        (true, rest)
    } else {
        return Err("bot URL must start with ws:// or wss://".into());
    };
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return Err("bot URL must contain a host and no userinfo".into());
    }
    let (host, explicit_port) = if let Some(hostport) = authority.strip_prefix('[') {
        let close = hostport
            .find(']')
            .ok_or("IPv6 bot URL host is missing ']'")?;
        let host = &hostport[..close];
        let rest_port = &hostport[close + 1..];
        let port = if rest_port.is_empty() {
            None
        } else {
            Some(
                rest_port
                    .strip_prefix(':')
                    .ok_or("invalid IPv6 bot URL port")?
                    .parse::<u16>()
                    .map_err(|_| "invalid bot URL port")?,
            )
        };
        (host.to_string(), port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || port.is_empty() {
            return Err("invalid bot URL host or port".into());
        }
        (
            host.to_string(),
            Some(port.parse::<u16>().map_err(|_| "invalid bot URL port")?),
        )
    } else {
        (authority.to_string(), None)
    };
    if host.contains(':') && !authority.starts_with('[') {
        return Err("IPv6 bot URL hosts must use brackets".into());
    }
    let port = explicit_port.unwrap_or(if secure { 443 } else { 80 });
    let suffix = &rest[authority_end..];
    let path = if suffix.is_empty() {
        "/".to_string()
    } else if suffix.starts_with('?') {
        format!("/{suffix}")
    } else {
        suffix.to_string()
    };
    let authority_for_header = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    Ok(Endpoint {
        secure,
        host,
        port,
        authority: authority_for_header,
        path,
    })
}

struct WsClient {
    stream: Transport,
    buffered: VecDeque<u8>,
    fragments: Option<Vec<u8>>,
    rng: rand::rngs::ThreadRng,
}

impl WsClient {
    fn connect(url: &str) -> Result<Self, String> {
        let endpoint = parse_endpoint(url)?;
        let address = if endpoint.host.contains(':') {
            format!("[{}]:{}", endpoint.host, endpoint.port)
        } else {
            format!("{}:{}", endpoint.host, endpoint.port)
        };
        let tcp = address
            .to_socket_addrs()
            .map_err(|e| format!("resolve {}: {e}", address))?
            .next()
            .ok_or_else(|| format!("could not resolve {address}"))?;
        let tcp = TcpStream::connect_timeout(&tcp, Duration::from_secs(10))
            .map_err(|e| format!("connect {address}: {e}"))?;
        tcp.set_nodelay(true).ok();
        tcp.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| format!("set WebSocket read timeout: {e}"))?;
        tcp.set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| format!("set WebSocket write timeout: {e}"))?;

        let mut stream = Transport::new(tcp, &endpoint)?;
        let mut key_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let key = base64_encode(&key_bytes);
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            endpoint.path, endpoint.authority, key
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("WebSocket handshake write: {e}"))?;
        stream
            .flush()
            .map_err(|e| format!("WebSocket handshake flush: {e}"))?;
        let (headers, leftover) = read_http_headers(&mut stream)?;
        let status_ok = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .map(|code| code == "101")
            .unwrap_or(false);
        if !status_ok {
            return Err(format!("WebSocket handshake was rejected: {headers}"));
        }
        let expected = base64_encode(&sha1_digest(format!("{key}{HANDSHAKE_GUID}").as_bytes()));
        let accept = headers.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name.trim().eq_ignore_ascii_case("sec-websocket-accept"))
                .then_some(value.trim().to_string())
        });
        if accept.as_deref() != Some(expected.as_str()) {
            return Err("WebSocket handshake has an invalid Sec-WebSocket-Accept".into());
        }
        Ok(Self {
            stream,
            buffered: leftover.into(),
            fragments: None,
            rng: rand::thread_rng(),
        })
    }

    fn send_text(&mut self, text: &str) -> Result<(), String> {
        self.send_frame(0x1, text.as_bytes())
    }

    fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> Result<(), String> {
        if payload.len() as u64 > MAX_FRAME_PAYLOAD {
            return Err("WebSocket payload is too large".into());
        }
        let mut frame = Vec::with_capacity(payload.len() + 14);
        frame.push(0x80 | opcode);
        let len = payload.len() as u64;
        if len < 126 {
            frame.push(0x80 | len as u8);
        } else if len <= u64::from(u16::MAX) {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&len.to_be_bytes());
        }
        let mut mask = [0u8; 4];
        self.rng.fill_bytes(&mut mask);
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(i, byte)| byte ^ mask[i % 4]),
        );
        self.stream
            .write_all(&frame)
            .map_err(|e| format!("WebSocket write: {e}"))?;
        self.stream
            .flush()
            .map_err(|e| format!("WebSocket flush: {e}"))
    }

    fn receive_text(&mut self) -> Result<Option<String>, String> {
        loop {
            let frame = self.read_frame()?;
            match frame.opcode {
                0x0 => {
                    let Some(mut fragments) = self.fragments.take() else {
                        return Err("unexpected WebSocket continuation frame".into());
                    };
                    fragments.extend(frame.payload);
                    if frame.fin {
                        let text = String::from_utf8(fragments)
                            .map_err(|_| "server sent non-UTF-8 WebSocket text".to_string())?;
                        return Ok(Some(text));
                    }
                    self.fragments = Some(fragments);
                }
                0x1 => {
                    if frame.fin {
                        let text = String::from_utf8(frame.payload)
                            .map_err(|_| "server sent non-UTF-8 WebSocket text".to_string())?;
                        return Ok(Some(text));
                    }
                    if self.fragments.is_some() {
                        return Err("nested WebSocket fragmented messages".into());
                    }
                    self.fragments = Some(frame.payload);
                }
                0x8 => {
                    let _ = self.send_frame(0x8, &frame.payload);
                    return Ok(None);
                }
                0x9 => self.send_frame(0xA, &frame.payload)?,
                0xA => {}
                _ => return Err(format!("unsupported WebSocket opcode 0x{:x}", frame.opcode)),
            }
        }
    }

    fn read_frame(&mut self) -> Result<Frame, String> {
        let first = self.read_byte()?;
        let second = self.read_byte()?;
        if first & 0x70 != 0 {
            return Err("WebSocket frame has unsupported RSV bits".into());
        }
        let fin = first & 0x80 != 0;
        let opcode = first & 0x0f;
        let masked = second & 0x80 != 0;
        let short = second & 0x7f;
        let length = match short {
            0..=125 => u64::from(short),
            126 => u64::from(u16::from_be_bytes([self.read_byte()?, self.read_byte()?])),
            127 => {
                let mut bytes = [0u8; 8];
                for byte in &mut bytes {
                    *byte = self.read_byte()?;
                }
                u64::from_be_bytes(bytes)
            }
            _ => return Err("invalid WebSocket payload length marker".into()),
        };
        if length > MAX_FRAME_PAYLOAD {
            return Err("WebSocket frame payload is too large".into());
        }
        let mut mask = [0u8; 4];
        if masked {
            for byte in &mut mask {
                *byte = self.read_byte()?;
            }
        }
        let mut payload = vec![0u8; length as usize];
        self.read_exact(&mut payload)?;
        if masked {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        Ok(Frame {
            fin,
            opcode,
            payload,
        })
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if let Some(byte) = self.buffered.pop_front() {
            return Ok(byte);
        }
        let mut byte = [0u8; 1];
        self.stream
            .read_exact(&mut byte)
            .map_err(|e| format!("WebSocket read: {e}"))?;
        Ok(byte[0])
    }

    fn read_exact(&mut self, destination: &mut [u8]) -> Result<(), String> {
        let mut offset = 0;
        while offset < destination.len() {
            if let Some(byte) = self.buffered.pop_front() {
                destination[offset] = byte;
                offset += 1;
                continue;
            }
            let read = self
                .stream
                .read(&mut destination[offset..])
                .map_err(|e| format!("WebSocket read: {e}"))?;
            if read == 0 {
                return Err("WebSocket closed while reading a frame".into());
            }
            offset += read;
        }
        Ok(())
    }
}

struct Frame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

enum Transport {
    Plain(TcpStream),
    #[cfg(feature = "battle-wss")]
    Tls(rustls::StreamOwned<rustls::ClientConnection, TcpStream>),
}

impl Transport {
    fn new(tcp: TcpStream, endpoint: &Endpoint) -> Result<Self, String> {
        if !endpoint.secure {
            return Ok(Self::Plain(tcp));
        }
        #[cfg(feature = "battle-wss")]
        {
            // The ring provider is explicitly installed so this remains
            // deterministic even when another dependency installs a provider
            // first in the process.
            let _ = rustls::crypto::ring::default_provider().install_default();
            let roots =
                rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let server_name = rustls::pki_types::ServerName::try_from(endpoint.host.clone())
                .map_err(|_| format!("invalid TLS server name {:?}", endpoint.host))?;
            let connection = rustls::ClientConnection::new(Arc::new(config), server_name)
                .map_err(|e| format!("TLS connection setup: {e}"))?;
            return Ok(Self::Tls(rustls::StreamOwned::new(connection, tcp)));
        }
        #[cfg(not(feature = "battle-wss"))]
        {
            let _ = tcp;
            Err("wss:// support was disabled; rebuild with --features battle-wss".into())
        }
    }
}

impl Read for Transport {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buffer),
            #[cfg(feature = "battle-wss")]
            Self::Tls(stream) => stream.read(buffer),
        }
    }
}

impl Write for Transport {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buffer),
            #[cfg(feature = "battle-wss")]
            Self::Tls(stream) => stream.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            #[cfg(feature = "battle-wss")]
            Self::Tls(stream) => stream.flush(),
        }
    }
}

fn read_http_headers(stream: &mut Transport) -> Result<(String, Vec<u8>), String> {
    let mut bytes = Vec::with_capacity(1024);
    let mut scratch = [0u8; 1024];
    loop {
        let read = stream
            .read(&mut scratch)
            .map_err(|e| format!("WebSocket handshake read: {e}"))?;
        if read == 0 {
            return Err("WebSocket peer closed during handshake".into());
        }
        bytes.extend_from_slice(&scratch[..read]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let body_start = end + 4;
            let header = String::from_utf8(bytes[..end].to_vec())
                .map_err(|_| "WebSocket handshake was not valid UTF-8".to_string())?;
            return Ok((header, bytes[body_start..].to_vec()));
        }
        if bytes.len() > MAX_HTTP_HEADERS {
            return Err("WebSocket handshake headers are too large".into());
        }
    }
}

fn sha1_digest(input: &[u8]) -> [u8; 20] {
    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x67452301u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, bytes) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut digest = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        result.push(ALPHABET[(a >> 2) as usize] as char);
        result.push(ALPHABET[((a & 0x03) << 4 | b >> 4) as usize] as char);
        result.push(if chunk.len() > 1 {
            ALPHABET[((b & 0x0f) << 2 | c >> 6) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            ALPHABET[(c & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    result
}

impl fmt::Display for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}://{}{}",
            if self.secure { "wss" } else { "ws" },
            self.authority,
            self.path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_and_base64_match_websocket_handshake_example() {
        assert_eq!(
            base64_encode(&sha1_digest(
                b"dGhlIHNhbXBsZSBub25jZQ==258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
            )),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn parses_ws_urls_and_default_ports() {
        assert_eq!(
            parse_endpoint("ws://localhost/ws?matchId=m&role=ai").unwrap(),
            Endpoint {
                secure: false,
                host: "localhost".into(),
                port: 80,
                authority: "localhost:80".into(),
                path: "/ws?matchId=m&role=ai".into(),
            }
        );
        assert_eq!(parse_endpoint("wss://[::1]/").unwrap().port, 443);
    }

    #[test]
    fn protocol_rotation_is_clockwise_numbered() {
        assert_eq!(protocol_rotation(Rotation::North), 0);
        assert_eq!(protocol_rotation(Rotation::East), 1);
        assert_eq!(protocol_rotation(Rotation::South), 2);
        assert_eq!(protocol_rotation(Rotation::West), 3);
    }

    #[test]
    fn parses_visible_bottom_up_board_and_state() {
        let rows = (0..20)
            .map(|y| {
                if y == 0 {
                    "1000000000".to_string()
                } else {
                    "0000000000".to_string()
                }
            })
            .collect::<Vec<_>>();
        let state = json!({
            "boardRows": rows,
            "active": "T",
            "hold": null,
            "next": ["I", "O", "S", "Z", "J"],
            "canHold": true,
            "incomingGarbage": 2,
            "score": 12,
            "lines": 1,
            "level": 1
        });
        let (game, active, next, hold, can_hold) = game_from_state(&state).unwrap();
        assert_eq!(active, Piece::T);
        assert_eq!(next, vec![Piece::I, Piece::O, Piece::S, Piece::Z, Piece::J]);
        assert_eq!(hold, None);
        assert!(can_hold);
        assert_eq!(game.queue.len(), 6);
        assert_eq!(game.pending.len(), 2);
        assert_ne!(game.board.cols[0] & 1, 0);
        assert_eq!(game.score, 12);
        assert_eq!(game.lines, 1);
    }

    #[test]
    fn decision_response_echoes_request_id_and_uses_protocol_coordinates() {
        let rows = vec!["0000000000"; 20];
        let request = json!({
            "type": "decision.request",
            "requestId": "r-test",
            "deadlineMs": 100,
            "state": {
                "boardRows": rows,
                "active": "T",
                "hold": null,
                "next": ["I", "O", "S", "Z", "J"],
                "canHold": true,
                "incomingGarbage": 0,
                "score": 0,
                "lines": 0,
                "level": 1
            }
        });
        let mut searcher = Searcher::new(true, None);
        let response = decision_response(
            &request,
            &mut searcher,
            Budget {
                milliseconds: 1,
                iterations: 0,
            },
        )
        .unwrap();
        assert_eq!(response["type"], "decision.response");
        assert_eq!(response["requestId"], "r-test");
        assert_eq!(response["action"]["kind"], "placement");
        assert!(response["action"]["rotation"].as_u64().unwrap() <= 3);
        assert!(response["action"]["x"].as_i64().unwrap() >= -2);
    }
}
