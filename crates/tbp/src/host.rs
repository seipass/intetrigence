//! TBP host: process lifecycle, line-delimited JSON I/O and message flow.
//!
//! Implements checklist A and B of `docs/attax-integration.md` §8:
//! spawn (`bun attax-tbp.ts`), `info` with a timeout, `rules` / `ready`,
//! `start` / `suggest` / `play` / `new_piece`, stderr capture, think-time
//! measurement and a clean `quit`.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::message::{
    BotMessage, HostMessage, InfoPayload, MessageError, Move, NewPiecePayload, PlayPayload,
    StartPayload, SuggestionPayload,
};

/// One `start` resend, recorded for the audit log
/// (`docs/attax-integration.md` §8-E3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartResend {
    pub reason: String,
    pub board_hash: u64,
    pub turn: u32,
}

#[derive(Clone, Debug)]
pub struct HostConfig {
    /// Program to spawn (`bun`).
    pub program: String,
    /// Arguments (`<attax>/tbp/attax-tbp.ts`).
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    /// Extra environment variables (for example `ATTAX_TBP_LOG`).
    pub env: Vec<(String, String)>,
    /// Per-message timeout. The referee waits for the bot (no starvation,
    /// `docs/s2-referee-protocol.md` §7); this is only the forfeit guard.
    pub timeout: Duration,
    /// Where to copy the child's stderr.
    pub stderr_path: Option<PathBuf>,
}

impl HostConfig {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            cwd: None,
            env: Vec::new(),
            timeout: Duration::from_secs(30),
            stderr_path: None,
        }
    }

    pub fn command_line(&self) -> String {
        let mut line = self.program.clone();
        for arg in &self.args {
            line.push(' ');
            if arg.contains(' ') {
                line.push('"');
                line.push_str(arg);
                line.push('"');
            } else {
                line.push_str(arg);
            }
        }
        line
    }
}

#[derive(Debug)]
pub enum HostError {
    Io(String),
    Timeout(Duration),
    Closed,
    Protocol(String),
    Message(MessageError),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::Io(message) => write!(f, "i/o error: {message}"),
            HostError::Timeout(timeout) => write!(f, "no response within {timeout:?}"),
            HostError::Closed => write!(f, "bot closed stdout"),
            HostError::Protocol(message) => write!(f, "protocol error: {message}"),
            HostError::Message(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<MessageError> for HostError {
    fn from(error: MessageError) -> Self {
        HostError::Message(error)
    }
}

enum ReaderEvent {
    Line(String),
    Eof,
    Error(String),
}

/// A running TBP bot process.
pub struct Host {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<ReaderEvent>,
    reader: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    pub config: HostConfig,
    pub info: InfoPayload,
    /// Wall-clock `suggest` -> `suggestion` time per request, in milliseconds.
    pub think_ms: Vec<f64>,
    /// `move_info` from the last suggestion.
    pub last_move_info: Option<crate::message::MoveInfo>,
    pub start_resends: Vec<StartResend>,
    pub messages_received: u64,
    closed: bool,
}

impl Host {
    /// Spawns the bot and completes the handshake (`info` -> `rules` -> `ready`).
    pub fn spawn(config: HostConfig) -> Result<Self, HostError> {
        let mut command = Command::new(&config.program);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &config.cwd {
            command.current_dir(cwd);
        }
        for (key, value) in &config.env {
            command.env(key, value);
        }
        let mut child = command
            .spawn()
            .map_err(|error| HostError::Io(format!("spawn {}: {error}", config.command_line())))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::Io("child stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::Io("child stdout unavailable".into()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| HostError::Io("child stderr unavailable".into()))?;

        let stderr_path = config.stderr_path.clone();
        let stderr_thread = thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stderr.read_to_end(&mut buffer);
            if let Some(path) = stderr_path {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, &buffer);
            }
        });

        let (tx, rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(ReaderEvent::Eof);
                        break;
                    }
                    Ok(_) => {
                        let text = line.trim_end_matches(['\r', '\n']).to_string();
                        if tx.send(ReaderEvent::Line(text)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(ReaderEvent::Error(error.to_string()));
                        break;
                    }
                }
            }
        });

        let mut host = Self {
            child,
            stdin,
            rx,
            reader: Some(reader),
            stderr_thread: Some(stderr_thread),
            config,
            info: InfoPayload::default(),
            think_ms: Vec::new(),
            last_move_info: None,
            start_resends: Vec::new(),
            messages_received: 0,
            closed: false,
        };

        host.info = host.expect_info()?;
        host.send(&HostMessage::Rules)?;
        host.expect_ready()?;
        Ok(host)
    }

    fn expect_info(&mut self) -> Result<InfoPayload, HostError> {
        match self.recv_bot_message()? {
            BotMessage::Info(info) => Ok(info),
            other => Err(HostError::Protocol(format!(
                "expected info first, got {other:?}"
            ))),
        }
    }

    fn expect_ready(&mut self) -> Result<(), HostError> {
        match self.recv_bot_message()? {
            BotMessage::Ready => Ok(()),
            other => Err(HostError::Protocol(format!(
                "expected ready after rules, got {other:?}"
            ))),
        }
    }

    /// Reads one non-empty line and parses it.
    pub fn recv_bot_message(&mut self) -> Result<BotMessage, HostError> {
        self.recv_bot_message_since(Instant::now())
    }

    fn recv_bot_message_since(&mut self, started: Instant) -> Result<BotMessage, HostError> {
        loop {
            match receive_event(&self.rx, self.config.timeout, started)? {
                ReaderEvent::Line(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    self.messages_received += 1;
                    return Ok(BotMessage::parse(&line)?);
                }
                ReaderEvent::Eof => return Err(HostError::Closed),
                ReaderEvent::Error(message) => return Err(HostError::Io(message)),
            }
        }
    }

    pub fn send(&mut self, message: &HostMessage) -> Result<(), HostError> {
        let line = serde_json::to_string(message)
            .map_err(|error| HostError::Protocol(format!("serialize: {error}")))?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|error| HostError::Io(format!("write: {error}")))
    }

    /// Sends `start` (initial state or a resend after garbage).
    pub fn start(&mut self, payload: &StartPayload, reason: &str, board_hash: u64, turn: u32) {
        if reason != "initial" {
            self.start_resends.push(StartResend {
                reason: reason.to_string(),
                board_hash,
                turn,
            });
        }
        // A failed write is surfaced by the next `suggest`; the referee treats
        // it as a forfeit there.
        let _ = self.send(&HostMessage::Start(payload.clone()));
    }

    /// Sends `stop` (cancel the bot's current search). Bots that do not use it
    /// ignore the message (attax) or suppress it in their adapter (clys).
    pub fn stop(&mut self) {
        let _ = self.send(&HostMessage::Stop);
    }

    /// Sends `suggest` and waits for `suggestion`, recording the round trip.
    pub fn suggest(&mut self) -> Result<SuggestionPayload, HostError> {
        let started = Instant::now();
        self.send(&HostMessage::Suggest)?;
        loop {
            match self.recv_bot_message_since(started)? {
                BotMessage::Suggestion(suggestion) => {
                    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                    self.think_ms.push(elapsed);
                    self.last_move_info = suggestion.move_info.clone();
                    return Ok(suggestion);
                }
                BotMessage::Info(_) | BotMessage::Ready => continue,
            }
        }
    }

    /// Sends `play`. The referee must send this for every adopted placement
    /// (`docs/attax-integration.md` §8-D4).
    pub fn play(&mut self, mv: Move) -> Result<(), HostError> {
        self.send(&HostMessage::Play(PlayPayload { mv }))
    }

    pub fn new_piece(&mut self, piece: crate::message::Piece) -> Result<(), HostError> {
        self.send(&HostMessage::NewPiece(NewPiecePayload { piece }))
    }

    /// Sends `quit` and waits for a clean exit (checklist A6).
    pub fn quit(&mut self) -> Result<(), HostError> {
        if self.closed {
            return Ok(());
        }
        self.send(&HostMessage::Quit)?;
        self.closed = true;
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = loop {
            match self.child.try_wait() {
                Ok(Some(status)) if status.success() => break Ok(()),
                Ok(Some(status)) => {
                    break Err(HostError::Protocol(format!("bot quit with {status}")));
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        break Err(HostError::Timeout(Duration::from_secs(5)));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break Err(HostError::Io(format!("wait for bot: {error}")));
                }
            }
        };
        self.join_threads();
        result
    }

    fn join_threads(&mut self) {
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn kill(&mut self) {
        if !self.closed {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.closed = true;
        }
        self.join_threads();
    }

    pub fn think_time_stats(&self) -> ThinkTimeStats {
        ThinkTimeStats::from_samples(&self.think_ms)
    }
}

fn receive_event(
    rx: &Receiver<ReaderEvent>,
    timeout: Duration,
    started: Instant,
) -> Result<ReaderEvent, HostError> {
    let remaining = timeout.saturating_sub(started.elapsed());
    if remaining.is_zero() {
        return Err(HostError::Timeout(timeout));
    }
    match rx.recv_timeout(remaining) {
        Ok(event) => Ok(event),
        Err(RecvTimeoutError::Timeout) => Err(HostError::Timeout(timeout)),
        Err(RecvTimeoutError::Disconnected) => Err(HostError::Closed),
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.closed = true;
        }
        self.join_threads();
    }
}

/// median / p95 / max of a think-time sample, in milliseconds.
///
/// Median: middle value, or the mean of the two middle values for an even
/// count. p95: nearest-rank (`sorted[ceil(0.95 * n) - 1]`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThinkTimeStats {
    pub count: usize,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

impl ThinkTimeStats {
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let count = sorted.len();
        let median = if count % 2 == 1 {
            sorted[count / 2]
        } else {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        };
        let rank = ((0.95 * count as f64).ceil() as usize).clamp(1, count);
        Self {
            count,
            median_ms: median,
            p95_ms: sorted[rank - 1],
            max_ms: sorted[count - 1],
            mean_ms: sorted.iter().sum::<f64>() / count as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_messages_do_not_extend_the_response_deadline() {
        let (tx, rx) = mpsc::channel();
        tx.send(ReaderEvent::Line(String::new())).unwrap();
        let timeout = Duration::from_secs(1);
        let started = Instant::now() - Duration::from_secs(2);
        assert!(
            matches!(receive_event(&rx, timeout, started), Err(HostError::Timeout(value)) if value == timeout)
        );
    }

    #[test]
    fn think_time_stats_use_nearest_rank_p95() {
        let samples: Vec<f64> = (1..=100).map(|value| value as f64).collect();
        let stats = ThinkTimeStats::from_samples(&samples);
        assert_eq!(stats.count, 100);
        assert_eq!(stats.median_ms, 50.5);
        assert_eq!(stats.p95_ms, 95.0);
        assert_eq!(stats.max_ms, 100.0);
    }

    #[test]
    fn empty_think_time_stats_are_zero() {
        let stats = ThinkTimeStats::from_samples(&[]);
        assert_eq!(stats.count, 0);
        assert_eq!(stats.median_ms, 0.0);
    }

    #[test]
    fn command_line_quotes_spaces() {
        let config = HostConfig::new("bun", vec!["C:/a b/attax-tbp.ts".into()]);
        assert_eq!(config.command_line(), "bun \"C:/a b/attax-tbp.ts\"");
    }
}
