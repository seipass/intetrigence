//! TBP message types (`docs/attax-integration.md` §3.1).
//!
//! The wire format is one JSON object per line. Host-to-bot messages are
//! serialized from [`HostMessage`]; bot-to-host messages are parsed leniently
//! from [`serde_json::Value`] so an unknown `type` can be reported instead of
//! aborting the whole process.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use intetrigence_core::{PieceKind, SpinType};

/// Tetromino name used on the wire (`"I"`, `"O"`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Piece {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Piece {
    pub const ALL: [Piece; 7] = [
        Piece::I,
        Piece::O,
        Piece::T,
        Piece::S,
        Piece::Z,
        Piece::J,
        Piece::L,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Piece::I => "I",
            Piece::O => "O",
            Piece::T => "T",
            Piece::S => "S",
            Piece::Z => "Z",
            Piece::J => "J",
            Piece::L => "L",
        }
    }

    pub const fn to_core(self) -> PieceKind {
        match self {
            Piece::I => PieceKind::I,
            Piece::O => PieceKind::O,
            Piece::T => PieceKind::T,
            Piece::S => PieceKind::S,
            Piece::Z => PieceKind::Z,
            Piece::J => PieceKind::J,
            Piece::L => PieceKind::L,
        }
    }

    pub const fn from_core(kind: PieceKind) -> Self {
        match kind {
            PieceKind::I => Piece::I,
            PieceKind::O => Piece::O,
            PieceKind::T => Piece::T,
            PieceKind::S => Piece::S,
            PieceKind::Z => Piece::Z,
            PieceKind::J => Piece::J,
            PieceKind::L => Piece::L,
        }
    }
}

/// SRS orientation name used by the Cold Clear / TBP location convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    North,
    East,
    South,
    West,
}

impl Orientation {
    pub const ALL: [Orientation; 4] = [
        Orientation::North,
        Orientation::East,
        Orientation::South,
        Orientation::West,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Orientation::North => "north",
            Orientation::East => "east",
            Orientation::South => "south",
            Orientation::West => "west",
        }
    }
}

/// Spin label carried by a TBP `move`. The referee never trusts it for scoring
/// (`docs/s2-referee-protocol.md` §4.2-5); it is recorded for the scoring audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpinLabel {
    None,
    Mini,
    Full,
}

impl SpinLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            SpinLabel::None => "none",
            SpinLabel::Mini => "mini",
            SpinLabel::Full => "full",
        }
    }

    pub const fn to_core(self) -> SpinType {
        match self {
            SpinLabel::None => SpinType::None,
            SpinLabel::Mini => SpinType::Mini,
            SpinLabel::Full => SpinType::Full,
        }
    }

    pub const fn from_core(spin: SpinType) -> Self {
        match spin {
            SpinType::None => SpinLabel::None,
            SpinType::Mini => SpinLabel::Mini,
            SpinType::Full => SpinLabel::Full,
        }
    }
}

/// A placement in the Cold Clear convention: `(x, y)` is the SRS true rotation
/// centre, `orientation` is one of four SRS states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    #[serde(rename = "type")]
    pub kind: Piece,
    pub orientation: Orientation,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub location: Location,
    pub spin: SpinLabel,
}

/// `info` payload sent by a bot right after startup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoPayload {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub features: Vec<String>,
}

/// `start` payload. `board[y][x]`, `y = 0` is the bottom row, non-null means
/// occupied (`docs/attax-integration.md` §3.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartPayload {
    pub board: Vec<Vec<Option<u8>>>,
    pub queue: Vec<Piece>,
    pub hold: Option<Piece>,
    pub combo: u32,
    pub back_to_back: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayPayload {
    #[serde(rename = "move")]
    pub mv: Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewPiecePayload {
    pub piece: Piece,
}

/// Search statistics reported by the bot (`attax-integration.md` §5.3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveInfo {
    #[serde(default)]
    pub nodes: u64,
    #[serde(default)]
    pub nps: u64,
    #[serde(default)]
    pub extra: Option<String>,
}

impl MoveInfo {
    /// Parses the millisecond figure out of `extra` (`"attax 37.0ms"`).
    pub fn extra_ms(&self) -> Option<f64> {
        let extra = self.extra.as_deref()?;
        let start = extra.find(|c: char| c.is_ascii_digit())?;
        let rest = &extra[start..];
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        rest[..end].parse().ok()
    }
}

/// `suggestion` payload: candidates in priority order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionPayload {
    pub moves: Vec<Move>,
    #[serde(default)]
    pub move_info: Option<MoveInfo>,
}

/// Host-to-bot messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum HostMessage {
    #[serde(rename = "rules")]
    Rules,
    #[serde(rename = "start")]
    Start(StartPayload),
    #[serde(rename = "suggest")]
    Suggest,
    #[serde(rename = "play")]
    Play(PlayPayload),
    #[serde(rename = "new_piece")]
    NewPiece(NewPiecePayload),
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "quit")]
    Quit,
}

/// Bot-to-host messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotMessage {
    Info(InfoPayload),
    Ready,
    Suggestion(SuggestionPayload),
}

impl BotMessage {
    /// Parses one line. Unknown `type` values are reported as protocol errors.
    pub fn parse(line: &str) -> Result<Self, MessageError> {
        let value: Value =
            serde_json::from_str(line).map_err(|error| MessageError::Json(error.to_string()))?;
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or(MessageError::MissingType)?;
        match kind {
            "info" => serde_json::from_value(value)
                .map(BotMessage::Info)
                .map_err(|error| MessageError::Json(error.to_string())),
            "ready" => Ok(BotMessage::Ready),
            "suggestion" => serde_json::from_value(value)
                .map(BotMessage::Suggestion)
                .map_err(|error| MessageError::Json(error.to_string())),
            other => Err(MessageError::UnknownType(other.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageError {
    Json(String),
    MissingType,
    UnknownType(String),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::Json(message) => write!(f, "invalid JSON: {message}"),
            MessageError::MissingType => write!(f, "message has no string `type` field"),
            MessageError::UnknownType(kind) => write!(f, "unknown message type `{kind}`"),
        }
    }
}

impl std::error::Error for MessageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_roundtrips_through_core() {
        for piece in Piece::ALL {
            assert_eq!(Piece::from_core(piece.to_core()), piece);
            assert_eq!(piece.as_str().len(), 1);
        }
    }

    #[test]
    fn spin_labels_roundtrip_through_core() {
        for label in [SpinLabel::None, SpinLabel::Mini, SpinLabel::Full] {
            assert_eq!(SpinLabel::from_core(label.to_core()), label);
        }
    }

    #[test]
    fn host_messages_serialize_with_type_tag() {
        assert_eq!(
            serde_json::to_string(&HostMessage::Rules).unwrap(),
            r#"{"type":"rules"}"#
        );
        assert_eq!(
            serde_json::to_string(&HostMessage::NewPiece(NewPiecePayload { piece: Piece::T }))
                .unwrap(),
            r#"{"type":"new_piece","piece":"T"}"#
        );
        let play = HostMessage::Play(PlayPayload {
            mv: Move {
                location: Location {
                    kind: Piece::T,
                    orientation: Orientation::South,
                    x: 4,
                    y: 0,
                },
                spin: SpinLabel::Full,
            },
        });
        let json = serde_json::to_string(&play).unwrap();
        assert!(json.contains(r#""type":"play""#));
        assert!(json.contains(r#""orientation":"south""#));
        assert!(json.contains(r#""spin":"full""#));
    }

    #[test]
    fn start_payload_matches_attax_shape() {
        let payload = StartPayload {
            board: vec![vec![None; 10]; 40],
            queue: vec![Piece::T, Piece::I],
            hold: None,
            combo: 0,
            back_to_back: false,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["board"].as_array().unwrap().len(), 40);
        assert_eq!(json["board"][0].as_array().unwrap().len(), 10);
        assert_eq!(json["queue"][0], "T");
        assert!(json["hold"].is_null());
        assert_eq!(json["combo"], 0);
        assert_eq!(json["back_to_back"], false);
    }

    #[test]
    fn bot_messages_parse() {
        let info = BotMessage::parse(
            r#"{"type":"info","name":"Attax","version":"overhaul/full","author":"attax","features":[]}"#,
        )
        .unwrap();
        assert_eq!(
            info,
            BotMessage::Info(InfoPayload {
                name: "Attax".into(),
                version: "overhaul/full".into(),
                author: "attax".into(),
                features: vec![],
            })
        );
        assert_eq!(
            BotMessage::parse(r#"{"type":"ready"}"#).unwrap(),
            BotMessage::Ready
        );
        let suggestion = BotMessage::parse(
            r#"{"type":"suggestion","moves":[{"location":{"type":"T","orientation":"north","x":4,"y":0},"spin":"none"}],"move_info":{"nodes":10,"nps":0,"extra":"attax 3.5ms"}}"#,
        )
        .unwrap();
        match suggestion {
            BotMessage::Suggestion(s) => {
                assert_eq!(s.moves.len(), 1);
                assert_eq!(s.moves[0].location.kind, Piece::T);
                assert_eq!(s.move_info.as_ref().unwrap().nodes, 10);
                assert_eq!(s.move_info.as_ref().unwrap().extra_ms(), Some(3.5));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(BotMessage::parse(r#"{"type":"nope"}"#).is_err());
        assert!(BotMessage::parse("not json").is_err());
    }
}
