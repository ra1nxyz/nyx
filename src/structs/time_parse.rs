use chrono::{Duration, Utc};
use regex::Regex;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeParseError {
    #[error("Invalid format. Use: 30s, 10m, 2h, 7d")]
    InvalidFormat,
    #[error("Number parse error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Invalid time unit")]
    InvalidUnit,
    #[error("Time value must be positive")]
    NegativeValue,
}

pub struct ParsedDuration {
    pub duration: Duration,
    pub original_string: String,
}

impl ParsedDuration {
    pub fn new(input: &str) -> Result<Self, TimeParseError> {
        let re = Regex::new(r"^(\d+)([smhd])$").map_err(|_| TimeParseError::InvalidFormat)?;

        let captures = re.captures(input)
            .ok_or(TimeParseError::InvalidFormat)?;

        let value: i64 = captures[1].parse()?;
        if value <= 0 {
            return Err(TimeParseError::NegativeValue);
        }

        let unit = &captures[2];
        let duration = match unit {
            "s" => Duration::seconds(value),
            "m" => Duration::minutes(value),
            "h" => Duration::hours(value),
            "d" => Duration::days(value),
            _ => return Err(TimeParseError::InvalidUnit),
        };

        Ok(ParsedDuration {
            duration,
            original_string: input.to_string(),
        })
    }

    pub fn as_chrono_duration(&self) -> Duration {
        self.duration
    }

    pub fn until_datetime(&self) -> chrono::DateTime<Utc> {
        Utc::now() + self.duration
    }

    pub fn human_readable(&self) -> String {
        let seconds = self.duration.num_seconds();

        if seconds < 60 {
            format!("{} seconds", seconds)
        } else if seconds < 3600 {
            format!("{} minutes", self.duration.num_minutes())
        } else if seconds < 86400 {
            format!("{} hours", self.duration.num_hours())
        } else {
            format!("{} days", self.duration.num_days())
        }
    }
}

pub fn parse_duration(input: &str) -> Result<Duration, TimeParseError> {
    let parsed = ParsedDuration::new(input)?;
    Ok(parsed.duration)
}