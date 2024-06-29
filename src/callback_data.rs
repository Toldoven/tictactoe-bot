use std::{fmt::Display, str::FromStr};

use crate::game::GameError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackData {
    Join,
    Restart,
    Place { x: usize, y: usize },
    Unknown,
}

impl Display for CallbackData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                CallbackData::Join => "join".to_string(),
                CallbackData::Place { x, y } => format!("place:{x}:{y}"),
                CallbackData::Unknown => "unknown".to_string(),
                CallbackData::Restart => "restart".to_string(),
            }
            .as_str(),
        )
    }
}

impl FromStr for CallbackData {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = match s {
            "join" => Self::Join,
            s if s.starts_with("place") => {
                let mut split = s.split(":").skip(1);
                let mut process_split = || {
                    split
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .ok_or(GameError::UnknownCommand)
                };
                Self::Place {
                    x: process_split()?,
                    y: process_split()?,
                }
            }
            "restart" => Self::Restart,
            _ => Self::Unknown,
        };
        Ok(value)
    }
}
