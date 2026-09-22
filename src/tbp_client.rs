//! Minimal TBP client for running an external opponent in the referee.
//!
//! The upstream bot does not receive a garbage message after a game starts.
//! The client keeps the TBP tree through ordinary `play`/`new_piece` events,
//! and only restarts it after a garbage rise changes the board. This preserves
//! the opponent's search state whenever the protocol can represent it.
use crate::game::Game;
use intetrigence_engine::data::Placement;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub struct TbpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    paused: bool,
}

impl TbpClient {
    pub fn spawn(path: &Path, config: Option<&Path>) -> Result<Self, String> {
        let mut command = Command::new(path);
        if let Some(config) = config {
            command.args([
                "--config",
                config.to_str().ok_or("config path is not UTF-8")?,
            ]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {}: {e}", path.display()))?;
        let stdin = child.stdin.take().ok_or("TBP child stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("TBP child stdout unavailable")?;
        let mut client = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            paused: false,
        };
        let info = client.read_message()?;
        if info["type"] != "info" {
            return Err(format!("TBP child did not send info: {info}"));
        }
        client.send(json!({"type": "rules", "randomizer": "seven_bag"}))?;
        let ready = client.read_message()?;
        if ready["type"] != "ready" {
            return Err(format!("TBP child did not become ready: {ready}"));
        }
        Ok(client)
    }

    /// Start calculation from the referee's complete current state.
    pub fn start(&mut self, game: &Game) -> Result<(), String> {
        self.send_start(game)
    }

    /// Reset the child with the referee's complete current state.
    pub fn resync(&mut self, game: &Game) -> Result<(), String> {
        self.send(json!({"type": "stop"}))?;
        self.send_start(game)
    }

    /// Start the child and immediately suspend it. This is used by the
    /// strict referee so the child cannot search while the other player is
    /// using its thinking window.
    pub fn start_paused(&mut self, game: &Game) -> Result<(), String> {
        self.start(game)?;
        self.pause()
    }

    /// Resynchronize a suspended child and leave it suspended after the
    /// start message has been accepted by its TBP loop.
    pub fn resync_paused(&mut self, game: &Game) -> Result<(), String> {
        self.resume()?;
        self.send(json!({"type": "stop"}))?;
        self.send_start(game)?;
        self.send(json!({"type": "suggest"}))?;
        let response = self.read_message()?;
        if response["type"] != "suggestion" {
            return Err(format!("TBP child did not accept resync: {response}"));
        }
        self.pause()
    }

    /// Suspend or resume the external worker. The strict mode is currently
    /// available on Unix hosts, where the referee can suspend the whole TBP
    /// process without changing its search tree.
    pub fn pause(&mut self) -> Result<(), String> {
        if self.paused {
            return Ok(());
        }
        signal_child(&self.child, "-STOP")?;
        self.paused = true;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), String> {
        if !self.paused {
            return Ok(());
        }
        signal_child(&self.child, "-CONT")?;
        self.paused = false;
        Ok(())
    }

    /// Give the external process exactly one referee thinking window while
    /// the other player is idle, then suspend it before returning. The small
    /// suggestion exchange happens after the window and is not counted as a
    /// second search interval.
    pub fn choose_sliced(
        &mut self,
        game: &Game,
        milliseconds: u64,
    ) -> Result<(Option<Placement>, u64, u128), String> {
        if milliseconds == 0 {
            return Err("strict external evaluation requires positive --ms".into());
        }
        if !self.paused {
            return Err("strict external evaluation requires a paused child".into());
        }
        self.resume()?;
        let started = Instant::now();
        thread::sleep(Duration::from_millis(milliseconds));
        let result = self.suggest_once(game, started);
        let pause_result = self.pause();
        match (result, pause_result) {
            (Ok((mv, nodes, _)), Ok(())) => Ok((mv, nodes, started.elapsed().as_micros())),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }

    /// Tell the child about a move and the random pieces appended to its
    /// queue. The count is two only when an empty hold consumes both the
    /// active piece and the next preview; otherwise one new piece is added.
    pub fn advance(&mut self, before: &Game, mv: Placement, after: &Game) -> Result<(), String> {
        self.send(json!({"type": "play", "move": mv}))?;
        let current = before
            .queue
            .front()
            .copied()
            .ok_or("empty queue before play")?;
        let added = usize::from(before.hold.is_none() && mv.location.piece != current) + 1;
        if after.queue.len() < added {
            return Err("queue shorter than appended-piece count".into());
        }
        let first = after.queue.len() - added;
        for piece in after.queue.iter().skip(first) {
            self.send(json!({"type": "new_piece", "piece": piece}))?;
        }
        Ok(())
    }

    /// Give the child one thinking interval, then request its current move.
    /// If the worker has not expanded a move yet, retry until the interval
    /// expires. The caller still validates the returned placement locally.
    pub fn choose(
        &mut self,
        game: &Game,
        milliseconds: u64,
    ) -> Result<(Option<Placement>, u64, u128), String> {
        if milliseconds == 0 {
            return Err("external TBP evaluation requires positive --ms".into());
        }
        let start = Instant::now();
        let deadline = start + Duration::from_millis(milliseconds);
        loop {
            let (mv, nodes, _) = self.suggest_once(game, start)?;
            if mv.is_some() {
                return Ok((mv, nodes, start.elapsed().as_micros()));
            }
            if Instant::now() >= deadline {
                return Ok((None, nodes, start.elapsed().as_micros()));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn suggest_once(
        &mut self,
        game: &Game,
        started: Instant,
    ) -> Result<(Option<Placement>, u64, u128), String> {
        self.send(json!({"type": "suggest"}))?;
        let response = self.read_message()?;
        if response["type"] != "suggestion" {
            return Err(format!("TBP child returned unexpected message: {response}"));
        }
        let moves: Vec<Placement> = serde_json::from_value(response["moves"].clone())
            .map_err(|e| format!("invalid TBP moves: {e}"))?;
        let nodes = response["move_info"]["nodes"].as_u64().unwrap_or(0);
        let mv = moves.into_iter().find(|mv| game.legal(*mv));
        Ok((mv, nodes, started.elapsed().as_micros()))
    }

    fn send_start(&mut self, game: &Game) -> Result<(), String> {
        self.send(json!({
            "type": "start",
            "board": game.board_json(),
            "queue": game.queue,
            "hold": game.hold,
            "combo": game.combo,
            "back_to_back": game.b2b,
            "randomizer": {
                "type": "seven_bag",
                "bag_state": game.bag,
            },
        }))
    }

    fn send(&mut self, value: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.stdin, &value).map_err(|e| e.to_string())?;
        self.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())
    }

    fn read_message(&mut self) -> Result<Value, String> {
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .map_err(|e| format!("read TBP child: {e}"))?;
        if read == 0 {
            let status = self.child.try_wait().map_err(|e| e.to_string())?;
            return Err(format!("TBP child exited before a response: {status:?}"));
        }
        serde_json::from_str(line.trim()).map_err(|e| format!("invalid TBP JSON: {e}"))
    }
}

impl Drop for TbpClient {
    fn drop(&mut self) {
        let _ = self.send(json!({"type": "quit"}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn signal_child(child: &Child, signal: &str) -> Result<(), String> {
    if cfg!(unix) {
        let status = std::process::Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .map_err(|e| format!("signal TBP child: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("signal TBP child failed with {status}"))
        }
    } else {
        let _ = (child, signal);
        Err("strict external timing requires a Unix host".into())
    }
}
