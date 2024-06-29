use std::{collections::HashMap, fmt::Display};

use color_eyre::eyre::Result;
use itertools::Itertools;
use shrinkwraprs::Shrinkwrap;
use teloxide::types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, User};

use crate::callback_data::CallbackData;

// use itertools::Itertools;

const BOARD_SIZE: usize = 3;

const MAX_INDEX: usize = BOARD_SIZE - 1;

const WIN_CONDITIONS: [[BoardIndex; BOARD_SIZE]; (BOARD_SIZE * 2) + 2] = [
    [BoardIndex(0, 0), BoardIndex(0, 1), BoardIndex(0, 2)],
    [BoardIndex(1, 0), BoardIndex(1, 1), BoardIndex(1, 2)],
    [BoardIndex(2, 0), BoardIndex(2, 1), BoardIndex(2, 2)],
    [BoardIndex(0, 1), BoardIndex(1, 1), BoardIndex(2, 1)],
    [BoardIndex(0, 0), BoardIndex(1, 0), BoardIndex(2, 0)],
    [BoardIndex(0, 2), BoardIndex(1, 2), BoardIndex(2, 2)],
    [BoardIndex(0, 0), BoardIndex(1, 1), BoardIndex(2, 2)],
    [BoardIndex(2, 0), BoardIndex(1, 1), BoardIndex(0, 2)],
];

#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Board(pub [[Option<Shape>; BOARD_SIZE]; BOARD_SIZE]);

impl Board {
    pub fn as_buttons(&self) -> InlineKeyboardMarkup {
        let buttons: Vec<Vec<InlineKeyboardButton>> = self
            .iter()
            .enumerate()
            .map(|(y, row)| {
                row.iter()
                    .map(|shape| match shape {
                        Some(shape) => shape.to_string(),
                        None => " ".to_string(),
                    })
                    .enumerate()
                    .map(|(x, shape_string)| {
                        InlineKeyboardButton::callback(
                            shape_string,
                            CallbackData::Place { x, y }.to_string(),
                        )
                    })
                    .collect()
            })
            .collect();

        InlineKeyboardMarkup::new(buttons)
    }
}

pub struct BoardIndex(pub usize, pub usize);

impl BoardIndex {
    fn new(x: usize, y: usize) -> Result<Self, GameError> {
        if x > MAX_INDEX || y > MAX_INDEX {
            return Err(GameError::OutOfBounds);
        }
        Ok(Self(x, y))
    }
}

#[derive(Debug)]
pub enum GameError {
    // Todo,
    OutOfBounds,
    AlreadyOccupied,
    UnknownIndex,
    IllegalState,
    Permission,
    NoData,
    UnknownCommand,
}

impl Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GameError::OutOfBounds => "The cell is out of bounds",
            GameError::AlreadyOccupied => "This cell is already occupied",
            GameError::UnknownIndex => "Unknown cell",
            GameError::IllegalState => "Something went wrong: Illegal state",
            GameError::Permission => "You don't have permission to do that",
            GameError::NoData => "No data",
            GameError::UnknownCommand => {
                "Command doesn't exist or is not appliable to current state"
            }
        })
    }
}

impl Board {
    pub fn empty() -> Self {
        Self([[None; BOARD_SIZE]; BOARD_SIZE])
    }

    pub fn check_win_condition(&self, condition: &[BoardIndex; BOARD_SIZE]) -> Option<Shape> {
        condition
            .iter()
            .map(|index| self.get_ref(index))
            .reduce(|a, b| {
                let a = a?;
                (a == b?).then_some(a)
            })
            .unwrap_or_default()
            .copied()
    }

    pub fn check_winner(&self) -> Option<Shape> {
        WIN_CONDITIONS
            .iter()
            .find_map(|condition| self.check_win_condition(condition))
    }

    pub fn check_draw(&self) -> bool {
        self.iter().flatten().all(|cell| cell.is_some())
    }

    pub fn get_ref(&self, index: &BoardIndex) -> Option<&Shape> {
        self[index.1][index.0].as_ref()
    }

    pub fn set_cell(&mut self, index: &BoardIndex, shape: Shape) {
        self[index.1][index.0] = Some(shape)
    }
}

impl Default for Board {
    fn default() -> Self {
        Board::empty()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shape {
    #[default]
    X,
    O,
}

impl Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Shape::X => "❌",
            Shape::O => "⭕️",
        })
    }
}

static SHAPES: [Shape; 2] = [Shape::X, Shape::O];

pub struct Game {
    board: Board,
    players: HashMap<Shape, User>,
    score: HashMap<User, usize>,
    state: GameState,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            board: Board::default(),
            players: HashMap::new(),
            score: HashMap::new(),
            state: GameState::Waiting,
        }
    }
}

impl Game {
    pub fn player_name(&self, shape: &Shape) -> String {
        self.players
            .get(&shape)
            .and_then(|player| Some(player.full_name()))
            .unwrap_or(shape.to_string())
    }

    pub fn finished_text(&self, result: &GameResult) -> String {
        let score = Itertools::intersperse(
            self.score
                .iter()
                .sorted_by_key(|(_, score)| *score)
                .map(|(user, score)| format!("{}: {score}", user.full_name()))
                .rev(),
            String::from("\n"),
        )
        .collect::<String>();

        let result = match result {
            GameResult::Victory { winner } => {
                format!("{} {} won!", winner, self.player_name(&winner))
            }
            GameResult::Draw => format!("Draw!"),
        };

        format!("{result}\n\n{score}")
    }

    pub fn as_message(&self) -> (String, InlineKeyboardMarkup) {
        let text = match &self.state {
            GameState::Waiting => match self.players.values().next() {
                Some(user) => format!("{} is waiting for the opponent", user.full_name()),
                None => format!("Waiting for players"),
            },
            GameState::Turn(shape) => format!("{} {}'s turn", shape, self.player_name(&shape)),
            GameState::Finished(result) => self.finished_text(result),
        };

        let keyboard = match &self.state {
            GameState::Waiting => {
                InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
                    "Join",
                    CallbackData::Join.to_string(),
                )]])
            }
            GameState::Turn(_) => self.get_board(),
            GameState::Finished(_) => {
                self.get_board()
                    .append_row(vec![InlineKeyboardButton::callback(
                        "Restart",
                        CallbackData::Restart.to_string(),
                    )])
            }
        };

        (text, keyboard)
    }

    pub fn add_user(&mut self, user: User) -> Result<(), GameError> {
        let shape = SHAPES
            .iter()
            .find(|shape| !self.players.contains_key(shape))
            .copied()
            .ok_or(GameError::IllegalState)?;

        if self.players.values().any(|value| value.id == user.id) {
            return Err(GameError::Permission);
        }

        self.score.insert(user.clone(), Default::default());
        self.players.insert(shape, user);

        Ok(())
    }

    pub fn process_callback(&mut self, q: CallbackQuery) -> Result<(), GameError> {
        match self.state {
            GameState::Waiting => self.process_callback_waiting(q),
            GameState::Turn(turn) => self.process_callback_turn(turn, q),
            GameState::Finished(_) => self.process_callback_finished(q),
        }
    }

    pub fn get_board(&self) -> InlineKeyboardMarkup {
        self.board.as_buttons()
    }

    fn process_callback_waiting(&mut self, q: CallbackQuery) -> Result<(), GameError> {
        if q.data.ok_or(GameError::NoData)?.parse::<CallbackData>()? != CallbackData::Join {
            return Err(GameError::UnknownCommand);
        }
        self.add_user(q.from)?;
        if self.players.len() >= 2 {
            self.state = GameState::Turn(Shape::X);
            self.board = Board::empty();
        }
        Ok(())
    }

    fn process_callback_finished(&mut self, q: CallbackQuery) -> Result<(), GameError> {
        if q.data.ok_or(GameError::NoData)?.parse::<CallbackData>()? != CallbackData::Restart {
            return Err(GameError::UnknownCommand);
        }

        if !self.players.values().any(|user| user.id == q.from.id) {
            return Err(GameError::Permission);
        }

        self.reset()?;

        Ok(())
    }

    fn process_callback_turn(
        &mut self,
        turn_shape: Shape,
        q: CallbackQuery,
    ) -> Result<(), GameError> {
        let shape_user = self
            .players
            .get(&turn_shape)
            .ok_or(GameError::IllegalState)?;

        if shape_user.id != q.from.id {
            return Err(GameError::Permission);
        }

        let index = match q.data.ok_or(GameError::NoData)?.parse::<CallbackData>()? {
            CallbackData::Place { x, y } => BoardIndex::new(x, y),
            _ => Err(GameError::IllegalState),
        }?;

        if self.board.get_ref(&index).is_some() {
            return Err(GameError::AlreadyOccupied);
        }

        self.board.set_cell(&index, turn_shape);

        kiam::when! {
            let Some(winner) = self.board.check_winner() => {
                if let Some(winner) = self.players.get(&winner) {
                    *self.score.entry(winner.clone()).or_insert(0) += 1;
                }
                self.state = GameState::Finished(GameResult::Victory { winner });
            },
            self.board.check_draw() => {
                self.state = GameState::Finished(GameResult::Draw);
            },
            _ => {
                self.state = GameState::Turn(match turn_shape {
                    Shape::O => Shape::X,
                    Shape::X => Shape::O,
                });
            }
        }

        Ok(())
    }

    fn reset(&mut self) -> Result<(), GameError> {
        self.swap_shapes()?;
        self.board = Board::default();
        self.state = GameState::Turn(Shape::default());
        Ok(())
    }

    fn swap_shapes(&mut self) -> Result<(), GameError> {
        let x = self
            .players
            .get_mut(&Shape::X)
            .ok_or(GameError::IllegalState)? as *mut User;
        let o = self
            .players
            .get_mut(&Shape::O)
            .ok_or(GameError::IllegalState)? as *mut User;
        unsafe {
            std::ptr::swap(x, o);
        }
        Ok(())
    }
}

pub enum GameState {
    Waiting,
    Turn(Shape),
    Finished(GameResult),
}

#[derive(Clone)]
pub enum GameResult {
    Victory { winner: Shape },
    Draw,
}
