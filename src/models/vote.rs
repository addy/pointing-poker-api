use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Vote {
    Zero,
    One,
    Two,
    Three,
    Five,
    Eight,
    Thirteen,
    TwentyOne,
    QuestionMark,
    Coffee,
    Hidden,
}

impl Vote {
    pub fn value(&self) -> Option<String> {
        match self {
            Vote::Zero => Some(0.to_string()),
            Vote::One => Some(1.to_string()),
            Vote::Two => Some(2.to_string()),
            Vote::Three => Some(3.to_string()),
            Vote::Five => Some(5.to_string()),
            Vote::Eight => Some(8.to_string()),
            Vote::Thirteen => Some(13.to_string()),
            Vote::TwentyOne => Some(21.to_string()),
            Vote::QuestionMark => Some("?".to_string()),
            Vote::Coffee => Some("coffee".to_string()),
            Vote::Hidden => None, // Special votes don't have numeric values
        }
    }

    pub fn from_string(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "0" => Ok(Vote::Zero),
            "1" => Ok(Vote::One),
            "2" => Ok(Vote::Two),
            "3" => Ok(Vote::Three),
            "5" => Ok(Vote::Five),
            "8" => Ok(Vote::Eight),
            "13" => Ok(Vote::Thirteen),
            "21" => Ok(Vote::TwentyOne),
            "?" => Ok(Vote::QuestionMark),
            "coffee" => Ok(Vote::Coffee),
            _ => Err(format!("Invalid vote value: {}", value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub value: String,
}
