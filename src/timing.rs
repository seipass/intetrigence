//! A deterministic frame-level Guideline movement validator.
//!
//! The versus arena intentionally uses lock placements because that is the
//! interface exposed by TBP. This module provides a separate timing model for
//! validating input sequences: movement, SRS rotation, gravity, lock delay,
//! and the finite move-reset budget are explicit and testable. Normal gravity
//! uses the published level table and soft drop uses the Guideline's 20x rate.
//! `Validator::new_guideline` also exposes the public 0.2-second Generation
//! Phase as twelve 60 Hz frames.
use crate::guideline::{hard_drop, is_obstructed, try_rotate_placement, try_shift};
use intetrigence_engine::data::{Board, Piece, PieceLocation, Placement, Rotation, Spin};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
};

/// Timing values for one reproducible validation profile.
///
/// Guideline games expose these values through their game mode; they are not
/// universal across every client. Keeping them in a profile makes the chosen
/// values explicit instead of silently treating one client as authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimingProfile {
    /// Whole cells of gravity applied during every frame.
    pub gravity_cells_per_frame: u8,
    /// Numerator for fractional gravity applied by the frame accumulator.
    pub gravity_numerator: u16,
    /// Denominator for fractional gravity applied by the frame accumulator.
    pub gravity_denominator: u16,
    /// Frames a grounded piece may remain before locking.
    pub lock_delay_frames: u16,
    /// Maximum grounded movement/rotation resets during one piece.
    pub max_lock_resets: u8,
    /// Soft-drop speed as a multiple of the normal gravity rate.
    pub soft_drop_multiplier: u8,
}

impl Default for TimingProfile {
    fn default() -> Self {
        Self::guideline_level(1)
    }
}

impl TimingProfile {
    /// Return the public Guideline fall-speed profile for levels 1 through 15.
    /// The published seconds-per-cell values are converted to fixed-point
    /// cells per 60 Hz frame; level 1 is one cell per second.
    pub fn guideline_level(level: u8) -> Self {
        const MILLIS_PER_CELL: [u16; 15] = [
            1000, 793, 618, 473, 355, 262, 190, 135, 94, 64, 43, 28, 18, 11, 7,
        ];
        let milliseconds = MILLIS_PER_CELL[level.clamp(1, 15) as usize - 1];
        let denominator = 60 * milliseconds;
        let gravity_cells_per_frame = 1000 / denominator;
        let gravity_numerator = 1000 % denominator;
        Self {
            gravity_cells_per_frame: gravity_cells_per_frame as u8,
            gravity_numerator,
            gravity_denominator: denominator,
            lock_delay_frames: 30,
            max_lock_resets: 15,
            soft_drop_multiplier: 20,
        }
    }

    /// Construct a profile useful for deterministic stress tests.
    pub const fn fixed_gravity(
        gravity_cells_per_frame: u8,
        lock_delay_frames: u16,
        max_lock_resets: u8,
    ) -> Self {
        Self {
            gravity_cells_per_frame,
            gravity_numerator: 0,
            gravity_denominator: 1,
            lock_delay_frames,
            max_lock_resets,
            soft_drop_multiplier: 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    None,
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    HardDrop,
}

/// A button snapshot before the Guideline auto-repeat policy turns it into a
/// frame action. Rotation and hard drop are edge-triggered; left/right and
/// soft drop may remain held across frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    pub left: bool,
    pub right: bool,
    pub soft_drop: bool,
    pub rotate_cw: bool,
    pub rotate_ccw: bool,
    pub hard_drop: bool,
}

/// Actions that can coexist in one 60 Hz frame. This keeps horizontal
/// auto-repeat and rotation independent, as required by the public controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameInput {
    pub horizontal: i8,
    pub rotate_cw: bool,
    pub rotate_ccw: bool,
    pub soft_drop: bool,
    pub hard_drop: bool,
}

impl From<Input> for FrameInput {
    fn from(input: Input) -> Self {
        match input {
            Input::None => Self::default(),
            Input::Left => Self {
                horizontal: -1,
                ..Self::default()
            },
            Input::Right => Self {
                horizontal: 1,
                ..Self::default()
            },
            Input::RotateCw => Self {
                rotate_cw: true,
                ..Self::default()
            },
            Input::RotateCcw => Self {
                rotate_ccw: true,
                ..Self::default()
            },
            Input::SoftDrop => Self {
                soft_drop: true,
                ..Self::default()
            },
            Input::HardDrop => Self {
                hard_drop: true,
                ..Self::default()
            },
        }
    }
}

/// Deterministic 60 Hz input-repeat values for the selected battle profile:
/// DAS 167 ms rounds to ten frames and ARR 33 ms rounds to two frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputProfile {
    pub auto_repeat_delay_frames: u16,
    pub auto_repeat_interval_frames: u16,
}

impl Default for InputProfile {
    fn default() -> Self {
        Self {
            auto_repeat_delay_frames: 10,
            auto_repeat_interval_frames: 2,
        }
    }
}

/// Convert held button snapshots into deterministic frame actions.
#[derive(Clone, Copy, Debug)]
pub struct InputRepeater {
    profile: InputProfile,
    left_age: u16,
    right_age: u16,
    active_direction: i8,
    left_delayed: bool,
    right_delayed: bool,
    previous_left: bool,
    previous_right: bool,
    previous_rotate_cw: bool,
    previous_rotate_ccw: bool,
    previous_hard_drop: bool,
}

impl InputRepeater {
    pub const fn new(profile: InputProfile) -> Self {
        Self {
            profile,
            left_age: 0,
            right_age: 0,
            active_direction: 0,
            left_delayed: false,
            right_delayed: false,
            previous_left: false,
            previous_right: false,
            previous_rotate_cw: false,
            previous_rotate_ccw: false,
            previous_hard_drop: false,
        }
    }

    pub fn next(&mut self, buttons: Buttons) -> FrameInput {
        let previous_direction = self.active_direction;
        let left_pressed = buttons.left && !self.previous_left;
        let right_pressed = buttons.right && !self.previous_right;
        let direction = match (buttons.left, buttons.right) {
            (false, false) => 0,
            (true, false) => -1,
            (false, true) => 1,
            (true, true) if right_pressed && !left_pressed => 1,
            (true, true) if left_pressed && !right_pressed => -1,
            (true, true) => previous_direction,
        };
        if direction != previous_direction {
            self.left_age = 0;
            self.right_age = 0;
            // Switching to the opposite direction while the original key is
            // still held must apply the initial delay again. This includes
            // releasing one of two held directions.
            let delayed = previous_direction != 0;
            self.left_delayed = delayed && direction == -1;
            self.right_delayed = delayed && direction == 1;
        }
        if direction == 0 {
            self.left_age = 0;
            self.right_age = 0;
            self.left_delayed = false;
            self.right_delayed = false;
        }
        self.active_direction = direction;
        let horizontal = match direction {
            -1 => Self::repeat(self.profile, &mut self.left_age, self.left_delayed, -1),
            1 => Self::repeat(self.profile, &mut self.right_age, self.right_delayed, 1),
            _ => 0,
        };
        let frame = FrameInput {
            horizontal,
            rotate_cw: buttons.rotate_cw && !self.previous_rotate_cw,
            rotate_ccw: buttons.rotate_ccw && !self.previous_rotate_ccw,
            soft_drop: buttons.soft_drop,
            hard_drop: buttons.hard_drop && !self.previous_hard_drop,
        };
        self.previous_left = buttons.left;
        self.previous_right = buttons.right;
        self.previous_rotate_cw = buttons.rotate_cw;
        self.previous_rotate_ccw = buttons.rotate_ccw;
        self.previous_hard_drop = buttons.hard_drop;
        frame
    }

    fn repeat(profile: InputProfile, age: &mut u16, delayed: bool, direction: i8) -> i8 {
        if *age == 0 {
            *age = 1;
            return if delayed { 0 } else { direction };
        }
        *age = age.saturating_add(1);
        let delay = profile.auto_repeat_delay_frames.max(1);
        let interval = profile.auto_repeat_interval_frames.max(1);
        if *age >= delay && (*age - delay).is_multiple_of(interval) {
            direction
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEvent {
    Generation {
        location: PieceLocation,
        remaining_frames: u16,
    },
    Active(PieceLocation),
    Locked(PieceLocation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimingError {
    SpawnBlocked,
    AlreadyLocked,
}

#[derive(Clone)]
pub struct Validator {
    board: Board,
    active: PieceLocation,
    spin: Spin,
    profile: TimingProfile,
    generation_frames: u16,
    gravity_accumulator: u32,
    lock_frames: u16,
    lock_resets: u8,
    lowest_y: i8,
    locked: bool,
}

impl Validator {
    pub fn new(board: Board, piece: Piece, profile: TimingProfile) -> Result<Self, TimingError> {
        Self::new_with_generation(board, piece, profile, 0)
    }

    /// Construct a validator with an explicit Generation Phase delay. The
    /// public 2009 profile uses twelve 60 Hz frames (0.2 seconds), while the
    /// zero-delay constructor is useful for placement-only callers.
    pub fn new_guideline(
        board: Board,
        piece: Piece,
        profile: TimingProfile,
    ) -> Result<Self, TimingError> {
        Self::new_with_generation(board, piece, profile, 12)
    }

    pub fn new_with_generation(
        board: Board,
        piece: Piece,
        profile: TimingProfile,
        generation_frames: u16,
    ) -> Result<Self, TimingError> {
        let mut active = PieceLocation {
            piece,
            rotation: Rotation::North,
            x: 4,
            y: 19,
        };
        if is_obstructed(&board, active) {
            active.y += 1;
            if is_obstructed(&board, active) {
                return Err(TimingError::SpawnBlocked);
            }
        }
        Ok(Self {
            board,
            active,
            spin: Spin::None,
            profile,
            generation_frames,
            gravity_accumulator: 0,
            lock_frames: 0,
            lock_resets: 0,
            lowest_y: active.cells().into_iter().map(|(_, y)| y).min().unwrap(),
            locked: false,
        })
    }

    pub fn board(&self) -> Board {
        self.board
    }

    pub fn active(&self) -> PieceLocation {
        self.active
    }

    pub fn placement(&self) -> Placement {
        Placement {
            location: self.active,
            spin: self.spin,
        }
    }

    pub fn lock_frames(&self) -> u16 {
        self.lock_frames
    }

    pub fn lock_resets(&self) -> u8 {
        self.lock_resets
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn generation_frames(&self) -> u16 {
        self.generation_frames
    }

    /// Advance exactly one frame and return whether the piece remains active
    /// or has locked. Invalid movement inputs are ignored, matching game
    /// input handling; only an invalid lifecycle operation is an error.
    pub fn step(&mut self, input: Input) -> Result<FrameEvent, TimingError> {
        self.step_frame(input.into())
    }

    /// Advance one frame with independently repeatable horizontal, rotation,
    /// and soft-drop actions. The order is horizontal movement, rotation, then
    /// gravity; a hard drop takes precedence and locks immediately.
    pub fn step_frame(&mut self, frame: FrameInput) -> Result<FrameEvent, TimingError> {
        if self.locked {
            return Err(TimingError::AlreadyLocked);
        }

        if self.generation_frames > 0 {
            self.generation_frames -= 1;
            return Ok(FrameEvent::Generation {
                location: self.active,
                remaining_frames: self.generation_frames,
            });
        }

        if frame.hard_drop {
            let dropped = hard_drop(&self.board, self.active);
            if dropped != self.active {
                self.spin = Spin::None;
            }
            self.active = dropped;
            return Ok(self.lock_now());
        }

        let grounded_before = self.grounded();
        let mut grounded_move = false;
        if frame.horizontal != 0 {
            grounded_move = try_shift(&self.board, self.active, frame.horizontal.clamp(-1, 1))
                .map(|next| {
                    self.active = next;
                    self.spin = Spin::None;
                })
                .is_some()
                && grounded_before;
        }
        if frame.rotate_cw || frame.rotate_ccw {
            let clockwise = frame.rotate_cw || !frame.rotate_ccw;
            grounded_move |= try_rotate_placement(&self.board, self.active, clockwise)
                .map(|next| {
                    self.active = next.location;
                    self.spin = next.spin;
                })
                .is_some()
                && grounded_before;
        }

        if grounded_move && self.lock_resets < self.profile.max_lock_resets {
            self.lock_resets += 1;
            self.lock_frames = 0;
        }

        // Fractional gravity is accumulated so a level-1 profile moves one
        // cell every 60 frames. The Guideline defines soft drop as twenty
        // times the normal fall speed, so it uses the same fixed-point rate
        // rather than an unrelated one-cell-per-frame shortcut.
        let denominator = u32::from(self.profile.gravity_denominator.max(1));
        let normal_rate = u32::from(self.profile.gravity_cells_per_frame) * denominator
            + u32::from(self.profile.gravity_numerator);
        let rate = if frame.soft_drop {
            normal_rate * u32::from(self.profile.soft_drop_multiplier.max(1))
        } else {
            normal_rate
        };
        let accumulated = self.gravity_accumulator.saturating_add(rate);
        let gravity_cells = accumulated / denominator;
        self.gravity_accumulator = accumulated % denominator;
        let before_gravity = self.active;
        for _ in 0..gravity_cells {
            if let Some(next) = try_shift(
                &self.board,
                PieceLocation {
                    y: self.active.y - 1,
                    ..self.active
                },
                0,
            ) {
                self.active = next;
            } else {
                break;
            }
        }
        if self.active != before_gravity {
            self.spin = Spin::None;
        }

        // Extended Placement grants a fresh reset budget when the piece
        // reaches a row below every row reached earlier. A rotation kick can
        // lift the piece and later let it descend again, so this must be
        // tracked independently of whether the piece is grounded this frame.
        let lowest_cell_y = self.lowest_cell_y();
        if lowest_cell_y < self.lowest_y {
            self.lowest_y = lowest_cell_y;
            self.lock_resets = 0;
            self.lock_frames = 0;
        }

        if !self.grounded() {
            if self.lock_resets < self.profile.max_lock_resets {
                self.lock_frames = 0;
            }
            return Ok(FrameEvent::Active(self.active));
        }

        if self.profile.lock_delay_frames == 0
            || self.lock_frames.saturating_add(1) >= self.profile.lock_delay_frames
        {
            Ok(self.lock_now())
        } else {
            self.lock_frames += 1;
            Ok(FrameEvent::Active(self.active))
        }
    }

    fn grounded(&self) -> bool {
        is_obstructed(
            &self.board,
            PieceLocation {
                y: self.active.y - 1,
                ..self.active
            },
        )
    }

    fn lowest_cell_y(&self) -> i8 {
        self.active
            .cells()
            .into_iter()
            .map(|(_, y)| y)
            .min()
            .unwrap()
    }

    fn lock_now(&mut self) -> FrameEvent {
        self.board.place(self.active);
        self.locked = true;
        FrameEvent::Locked(self.active)
    }
}

/// One human input frame recorded by the movement battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleInput {
    None,
    Hold,
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    HardDrop,
}

impl BattleInput {
    fn frame(self) -> FrameInput {
        match self {
            Self::None | Self::Hold => FrameInput::default(),
            Self::Left => FrameInput {
                horizontal: -1,
                ..FrameInput::default()
            },
            Self::Right => FrameInput {
                horizontal: 1,
                ..FrameInput::default()
            },
            Self::RotateCw => FrameInput {
                rotate_cw: true,
                ..FrameInput::default()
            },
            Self::RotateCcw => FrameInput {
                rotate_ccw: true,
                ..FrameInput::default()
            },
            Self::SoftDrop => FrameInput {
                soft_drop: true,
                ..FrameInput::default()
            },
            Self::HardDrop => FrameInput {
                hard_drop: true,
                ..FrameInput::default()
            },
        }
    }

    fn is_discrete(self) -> bool {
        matches!(
            self,
            Self::Left | Self::Right | Self::RotateCw | Self::RotateCcw
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MovementTraceFrame {
    pub input: BattleInput,
    pub active: Placement,
    pub phase: &'static str,
}

/// Replay a validated plan and expose the active placement after every frame.
///
/// The trace is intentionally derived from the same validator as replay
/// verification. Video renderers can therefore animate movement without
/// reimplementing SRS, gravity, or lock-delay rules.
pub fn trace_movement(
    board: Board,
    target: Placement,
    hold_used: bool,
    profile: TimingProfile,
    inputs: &[BattleInput],
) -> Result<Vec<MovementTraceFrame>, PlanError> {
    validate_movement_plan(board, target, hold_used, profile, inputs)?;
    let mut validator = Validator::new_guideline(board, target.location.piece, profile)
        .map_err(|_| PlanError::SpawnBlocked)?;
    let mut trace = Vec::with_capacity(inputs.len());
    for &input in inputs {
        let generation = validator.generation_frames() > 0;
        let event = validator
            .step_frame(input.frame())
            .map_err(|_| PlanError::InvalidInput)?;
        let phase = if generation {
            "generation"
        } else if matches!(event, FrameEvent::Locked(_)) {
            "locked"
        } else {
            "active"
        };
        trace.push(MovementTraceFrame {
            input,
            active: validator.placement(),
            phase,
        });
    }
    Ok(trace)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovementPlan {
    pub hold_used: bool,
    pub inputs: Vec<BattleInput>,
    pub locked: Placement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    SpawnBlocked,
    Unreachable,
    InvalidInput,
}

#[derive(Clone)]
struct PlanNode {
    validator: Validator,
    parent: Option<usize>,
    input: BattleInput,
    depth: u16,
    needs_release: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PlanKey {
    active: PieceLocation,
    spin: Spin,
    gravity_accumulator: u32,
    lock_frames: u16,
    lock_resets: u8,
    lowest_y: i8,
    needs_release: bool,
}

impl PlanKey {
    fn new(node: &PlanNode) -> Self {
        Self {
            active: node.validator.active,
            spin: node.validator.spin,
            gravity_accumulator: node.validator.gravity_accumulator,
            lock_frames: node.validator.lock_frames,
            lock_resets: node.validator.lock_resets,
            lowest_y: node.validator.lowest_y,
            needs_release: node.needs_release,
        }
    }
}

/// Find a human-executable 60 Hz input sequence for an AI-selected placement.
///
/// Discrete inputs require a release frame before another discrete input.
/// Soft drop may remain held. The returned sequence includes the twelve-frame
/// generation phase, an optional HOLD frame, and the final hard drop or
/// lock-delay frame.
pub fn plan_placement(
    board: Board,
    target: Placement,
    hold_used: bool,
    profile: TimingProfile,
) -> Result<MovementPlan, PlanError> {
    const MAX_DEPTH: u16 = 240;
    const MAX_NODES: usize = 500_000;

    let mut validator = Validator::new_guideline(board, target.location.piece, profile)
        .map_err(|_| PlanError::SpawnBlocked)?;
    let mut prefix = Vec::with_capacity(16);
    while validator.generation_frames() > 0 {
        validator
            .step_frame(FrameInput::default())
            .map_err(|_| PlanError::SpawnBlocked)?;
        prefix.push(BattleInput::None);
    }
    if hold_used {
        validator
            .step_frame(FrameInput::default())
            .map_err(|_| PlanError::SpawnBlocked)?;
        prefix.push(BattleInput::Hold);
    }

    let root = PlanNode {
        validator,
        parent: None,
        input: BattleInput::None,
        depth: 0,
        needs_release: false,
    };
    let mut nodes = vec![root];
    let mut queue = BinaryHeap::from([Reverse((plan_priority(&nodes[0], target), 0usize))]);
    let mut visited = HashSet::from([PlanKey::new(&nodes[0])]);

    while let Some(Reverse((_, index))) = queue.pop() {
        let node = nodes[index].clone();

        let mut dropped = node.validator.clone();
        if matches!(
            dropped.step_frame(BattleInput::HardDrop.frame()),
            Ok(FrameEvent::Locked(_))
        ) && same_placement(dropped.placement(), target)
        {
            let mut inputs = reconstruct_plan(&nodes, index, prefix);
            inputs.push(BattleInput::HardDrop);
            return Ok(MovementPlan {
                hold_used,
                inputs,
                locked: target,
            });
        }

        if node.depth >= MAX_DEPTH || nodes.len() >= MAX_NODES {
            continue;
        }
        let candidates: &[BattleInput] = if node.needs_release {
            &[BattleInput::None, BattleInput::SoftDrop]
        } else {
            &[
                BattleInput::Left,
                BattleInput::Right,
                BattleInput::RotateCw,
                BattleInput::RotateCcw,
                BattleInput::SoftDrop,
                BattleInput::None,
            ]
        };
        for &input in candidates {
            let mut next_validator = node.validator.clone();
            let event = match next_validator.step_frame(input.frame()) {
                Ok(event) => event,
                Err(_) => continue,
            };
            let next = PlanNode {
                validator: next_validator,
                parent: Some(index),
                input,
                depth: node.depth + 1,
                needs_release: input.is_discrete(),
            };
            if matches!(event, FrameEvent::Locked(_)) {
                if same_placement(next.validator.placement(), target) {
                    let inputs = reconstruct_plan_with_last(&nodes, index, prefix, input);
                    return Ok(MovementPlan {
                        hold_used,
                        inputs,
                        locked: target,
                    });
                }
                continue;
            }
            if visited.insert(PlanKey::new(&next)) {
                nodes.push(next);
                queue.push(Reverse((
                    plan_priority(nodes.last().expect("inserted plan node"), target),
                    nodes.len() - 1,
                )));
            }
        }
    }
    Err(PlanError::Unreachable)
}

/// Replay and validate a recorded movement plan against the same frame model.
pub fn validate_movement_plan(
    board: Board,
    target: Placement,
    hold_used: bool,
    profile: TimingProfile,
    inputs: &[BattleInput],
) -> Result<(), PlanError> {
    let mut validator = Validator::new_guideline(board, target.location.piece, profile)
        .map_err(|_| PlanError::SpawnBlocked)?;
    let mut seen_hold = false;
    let mut needs_release = false;
    for (index, &input) in inputs.iter().enumerate() {
        if validator.generation_frames() > 0 && input != BattleInput::None {
            return Err(PlanError::InvalidInput);
        }
        if input == BattleInput::Hold {
            if !hold_used || seen_hold || validator.generation_frames() > 0 {
                return Err(PlanError::InvalidInput);
            }
            seen_hold = true;
        } else if validator.generation_frames() == 0 && hold_used && !seen_hold {
            return Err(PlanError::InvalidInput);
        }
        if input.is_discrete() && needs_release {
            return Err(PlanError::InvalidInput);
        }
        let event = validator
            .step_frame(input.frame())
            .map_err(|_| PlanError::InvalidInput)?;
        needs_release = input.is_discrete();
        if matches!(event, FrameEvent::Locked(_)) && index + 1 != inputs.len() {
            return Err(PlanError::InvalidInput);
        }
    }
    if seen_hold != hold_used
        || !validator.is_locked()
        || !same_placement(validator.placement(), target)
    {
        return Err(PlanError::InvalidInput);
    }
    Ok(())
}

fn plan_priority(node: &PlanNode, target: Placement) -> u32 {
    let active = node.validator.active;
    let dropped = hard_drop(&node.validator.board, active).canonical_form();
    let target = target.location.canonical_form();
    let rotation_delta = (active.rotation as i32 - target.rotation as i32).unsigned_abs();
    let rotation_distance = rotation_delta.min(4 - rotation_delta);
    u32::from(node.depth)
        + active.x.abs_diff(target.x) as u32 * 4
        + dropped.y.abs_diff(target.y) as u32 * 2
        + rotation_distance * 3
}

fn same_placement(actual: Placement, target: Placement) -> bool {
    actual.location.canonical_form() == target.location.canonical_form()
        && actual.spin == target.spin
}

fn reconstruct_plan(
    nodes: &[PlanNode],
    mut index: usize,
    mut prefix: Vec<BattleInput>,
) -> Vec<BattleInput> {
    let mut reversed = Vec::new();
    while let Some(parent) = nodes[index].parent {
        reversed.push(nodes[index].input);
        index = parent;
    }
    reversed.reverse();
    prefix.extend(reversed);
    prefix
}

fn reconstruct_plan_with_last(
    nodes: &[PlanNode],
    index: usize,
    prefix: Vec<BattleInput>,
    last: BattleInput,
) -> Vec<BattleInput> {
    let mut result = reconstruct_plan(nodes, index, prefix);
    result.push(last);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guideline_auto_repeat_has_an_immediate_tap_then_delay() {
        let mut repeater = InputRepeater::new(InputProfile {
            auto_repeat_delay_frames: 3,
            auto_repeat_interval_frames: 2,
        });
        assert_eq!(
            repeater.next(Buttons {
                left: true,
                ..Buttons::default()
            }),
            FrameInput {
                horizontal: -1,
                ..FrameInput::default()
            }
        );
        for _ in 0..1 {
            assert_eq!(
                repeater.next(Buttons {
                    left: true,
                    ..Buttons::default()
                }),
                FrameInput::default()
            );
        }
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    ..Buttons::default()
                })
                .horizontal,
            -1
        );
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    ..Buttons::default()
                })
                .horizontal,
            0
        );
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    ..Buttons::default()
                })
                .horizontal,
            -1
        );
        assert_eq!(repeater.next(Buttons::default()), FrameInput::default());
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    ..Buttons::default()
                })
                .horizontal,
            -1
        );
    }

    #[test]
    fn rotations_and_hard_drop_are_edge_triggered_without_blocking_soft_drop() {
        let mut repeater = InputRepeater::new(InputProfile::default());
        let buttons = Buttons {
            rotate_cw: true,
            hard_drop: true,
            soft_drop: true,
            ..Buttons::default()
        };
        assert_eq!(
            repeater.next(buttons),
            FrameInput {
                rotate_cw: true,
                soft_drop: true,
                hard_drop: true,
                ..FrameInput::default()
            }
        );
        assert_eq!(
            repeater.next(buttons),
            FrameInput {
                soft_drop: true,
                ..FrameInput::default()
            }
        );
    }

    #[test]
    fn hard_drop_locks_on_the_lowest_valid_row() {
        let mut validator = Validator::new(Board::default(), Piece::T, TimingProfile::default())
            .expect("empty board has a valid spawn");
        assert_eq!(
            validator.step(Input::HardDrop).unwrap(),
            FrameEvent::Locked(PieceLocation {
                piece: Piece::T,
                rotation: Rotation::North,
                x: 4,
                y: 0,
            })
        );
        assert!(validator.is_locked());
        assert_eq!(validator.step(Input::None), Err(TimingError::AlreadyLocked));
    }

    #[test]
    fn guideline_generation_phase_delays_input_for_twelve_frames() {
        let mut validator =
            Validator::new_guideline(Board::default(), Piece::T, TimingProfile::default())
                .expect("empty board has a valid spawn");
        assert_eq!(validator.generation_frames(), 12);
        for remaining in (0..12).rev() {
            assert_eq!(
                validator.step(Input::HardDrop).unwrap(),
                FrameEvent::Generation {
                    location: validator.active(),
                    remaining_frames: remaining,
                }
            );
        }
        assert_eq!(validator.generation_frames(), 0);
        assert!(matches!(
            validator.step(Input::HardDrop).unwrap(),
            FrameEvent::Locked(_)
        ));
    }

    #[test]
    fn grounded_move_resets_lock_delay_only_until_the_cap() {
        let profile = TimingProfile::fixed_gravity(40, 2, 1);
        let mut validator = Validator::new(Board::default(), Piece::T, profile).unwrap();
        assert!(matches!(
            validator.step(Input::None).unwrap(),
            FrameEvent::Active(_)
        ));
        assert_eq!(validator.lock_frames(), 1);
        assert!(matches!(
            validator.step(Input::Left).unwrap(),
            FrameEvent::Active(_)
        ));
        assert_eq!(validator.lock_resets(), 1);
        assert_eq!(validator.lock_frames(), 1);
        assert!(matches!(
            validator.step(Input::Left).unwrap(),
            FrameEvent::Locked(_)
        ));
    }

    #[test]
    fn descending_below_the_previous_lowest_row_restores_extended_resets() {
        let profile = TimingProfile::fixed_gravity(0, 5, 1);
        let mut validator = Validator::new(Board::default(), Piece::T, profile).unwrap();
        // Model a piece that was kicked upward after touching the floor and
        // has already spent its reset. Reaching a new lower row must renew it.
        validator.active.y = 0;
        validator.lowest_y = 1;
        validator.lock_resets = 1;
        validator.lock_frames = 4;

        assert!(matches!(
            validator.step(Input::None).unwrap(),
            FrameEvent::Active(_)
        ));
        assert_eq!(validator.lowest_y, 0);
        assert_eq!(validator.lock_resets(), 0);
        assert_eq!(validator.lock_frames(), 1);
    }

    #[test]
    fn spawn_is_rejected_when_both_spawn_rows_are_blocked() {
        let mut board = Board::default();
        for y in 19..=22 {
            board.cols[3] |= 1 << y;
            board.cols[4] |= 1 << y;
            board.cols[5] |= 1 << y;
        }
        assert!(matches!(
            Validator::new(board, Piece::T, TimingProfile::default()),
            Err(TimingError::SpawnBlocked)
        ));
    }

    #[test]
    fn guideline_level_one_falls_one_cell_per_second_at_sixty_hz() {
        let mut validator = Validator::new(Board::default(), Piece::T, TimingProfile::default())
            .expect("empty board has a valid spawn");
        let spawn_y = validator.active().y;
        for _ in 0..59 {
            assert!(matches!(
                validator.step(Input::None).unwrap(),
                FrameEvent::Active(_)
            ));
        }
        assert_eq!(validator.active().y, spawn_y);
        assert!(matches!(
            validator.step(Input::None).unwrap(),
            FrameEvent::Active(_)
        ));
        assert_eq!(validator.active().y, spawn_y - 1);
    }

    #[test]
    fn soft_drop_uses_twenty_times_the_normal_gravity_rate() {
        let profile = TimingProfile::guideline_level(1);
        let mut normal = Validator::new(Board::default(), Piece::T, profile).unwrap();
        let mut soft = Validator::new(Board::default(), Piece::T, profile).unwrap();
        let normal_spawn = normal.active().y;
        let soft_spawn = soft.active().y;

        for _ in 0..3 {
            assert!(matches!(
                normal.step(Input::None).unwrap(),
                FrameEvent::Active(_)
            ));
            assert!(matches!(
                soft.step(Input::SoftDrop).unwrap(),
                FrameEvent::Active(_)
            ));
        }

        assert_eq!(normal.active().y, normal_spawn);
        assert_eq!(soft.active().y, soft_spawn - 1);
    }

    #[test]
    fn guideline_gravity_increases_through_level_fifteen() {
        let mut previous = TimingProfile::guideline_level(1);
        for level in 2..=15 {
            let current = TimingProfile::guideline_level(level);
            let previous_rate = u32::from(previous.gravity_cells_per_frame)
                * u32::from(previous.gravity_denominator)
                + u32::from(previous.gravity_numerator);
            let current_rate = u32::from(current.gravity_cells_per_frame)
                * u32::from(current.gravity_denominator)
                + u32::from(current.gravity_numerator);
            assert!(
                current_rate * u32::from(previous.gravity_denominator)
                    >= previous_rate * u32::from(current.gravity_denominator),
                "level {level} must fall at least as fast as the previous level"
            );
            previous = current;
        }
    }

    #[test]
    fn opposite_direction_restarts_with_the_initial_delay() {
        let mut repeater = InputRepeater::new(InputProfile {
            auto_repeat_delay_frames: 3,
            auto_repeat_interval_frames: 2,
        });
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    ..Buttons::default()
                })
                .horizontal,
            -1
        );
        assert_eq!(
            repeater
                .next(Buttons {
                    left: true,
                    right: true,
                    ..Buttons::default()
                })
                .horizontal,
            0
        );
        assert_eq!(
            repeater
                .next(Buttons {
                    right: true,
                    ..Buttons::default()
                })
                .horizontal,
            0
        );
        assert_eq!(
            repeater
                .next(Buttons {
                    right: true,
                    ..Buttons::default()
                })
                .horizontal,
            1
        );
    }

    #[test]
    fn placement_plan_executes_through_generation_and_human_inputs() {
        let board = Board::default();
        let target = intetrigence_engine::movegen::find_moves(&board, Piece::T)
            .into_iter()
            .map(|(placement, _)| placement)
            .filter(|placement| placement.spin == Spin::None)
            .max_by_key(|placement| placement.location.x)
            .unwrap();
        let profile = TimingProfile::guideline_level(1);
        let plan = plan_placement(board, target, false, profile).unwrap();
        assert!(plan.inputs.len() >= 13);
        assert_eq!(plan.inputs.last(), Some(&BattleInput::HardDrop));
        assert!(plan.inputs.contains(&BattleInput::Right));
        validate_movement_plan(board, target, false, profile, &plan.inputs).unwrap();
    }

    #[test]
    fn movement_validation_rejects_inputs_during_generation() {
        let board = Board::default();
        let target = intetrigence_engine::movegen::find_moves(&board, Piece::I)[0].0;
        let profile = TimingProfile::guideline_level(1);
        let mut plan = plan_placement(board, target, true, profile).unwrap();
        assert!(plan.inputs.contains(&BattleInput::Hold));
        plan.inputs[0] = BattleInput::Left;
        assert_eq!(
            validate_movement_plan(board, target, true, profile, &plan.inputs),
            Err(PlanError::InvalidInput)
        );
    }
}
