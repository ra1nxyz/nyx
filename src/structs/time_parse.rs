use std::num::ParseIntError;
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, TimeZone, Utc};
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeParseError {
    #[error("Invalid format. Use a relative time (30s, 10m, 2h, 7d, 2w, 3mo, 1y) or a date (14/05, 11/09/2027)")]
    InvalidFormat,
    #[error("Number parse error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Time value must be positive")]
    NegativeValue,
    #[error("That date doesn't exist")]
    InvalidDate,
    #[error("That date/time is in the past")]
    InThePast,
}

pub enum RelativeAmount {
    Duration(Duration),
    Months(u32),
}

// weird stuff happening with cargo, force recompile
pub enum ParsedWhen {
    Relative(RelativeAmount),
    Absolute(DateTime<Utc>),
}

impl ParsedWhen {
    pub fn new(input: &str) -> Result<Self, TimeParseError> {
        let input = input.trim();

        if let Some(parsed) = Self::try_parse_relative(input)? {
            return Ok(parsed);
        }
        if let Some(parsed) = Self::try_parse_absolute(input)? {
            return Ok(parsed);
        }

        Err(TimeParseError::InvalidFormat)
    }

    fn try_parse_relative(input: &str) -> Result<Option<Self>, TimeParseError> {
        // "mo" must be an explicit alternative as it won't fall out of "m" + leftover "o"
        let re = Regex::new(r"^(\d+)(mo|w|y|s|m|h|d)$").unwrap();
        let Some(captures) = re.captures(input) else {
            return Ok(None);
        };

        let value: i64 = captures[1].parse()?;
        if value <= 0 {
            return Err(TimeParseError::NegativeValue);
        }

        let amount = match &captures[2] {
            "s" => RelativeAmount::Duration(Duration::seconds(value)),
            "m" => RelativeAmount::Duration(Duration::minutes(value)),
            "h" => RelativeAmount::Duration(Duration::hours(value)),
            "d" => RelativeAmount::Duration(Duration::days(value)),
            "w" => RelativeAmount::Duration(Duration::weeks(value)),
            "mo" => RelativeAmount::Months(value as u32),
            "y" => RelativeAmount::Months(value as u32 * 12),
            _ => unreachable!(),
        };

        Ok(Some(ParsedWhen::Relative(amount)))
    }

    fn try_parse_absolute(input: &str) -> Result<Option<Self>, TimeParseError> {
        // DD/MM/YYYY
        if let Ok(date) = NaiveDate::parse_from_str(input, "%d/%m/%Y") {
            return Ok(Some(ParsedWhen::Absolute(Self::date_to_utc(date)?)));
        }

        // DD/MM -> current year -> next year if it's already passed
        let re = Regex::new(r"^(\d{1,2})/(\d{1,2})$").unwrap();
        if let Some(captures) = re.captures(input) {
            let day: u32 = captures[1].parse()?;
            let month: u32 = captures[2].parse()?;
            let now = Utc::now();

            let this_year = NaiveDate::from_ymd_opt(now.year(), month, day)
                .ok_or(TimeParseError::InvalidDate)?;
            let candidate = Self::date_to_utc(this_year)?;

            let date = if candidate <= now {
                NaiveDate::from_ymd_opt(now.year() + 1, month, day)
                    .ok_or(TimeParseError::InvalidDate)?
            } else {
                this_year
            };

            return Ok(Some(ParsedWhen::Absolute(Self::date_to_utc(date)?)));
        }

        Ok(None)
    }

    // Date input has no time component so default to 12:00 UTC
    fn date_to_utc(date: NaiveDate) -> Result<DateTime<Utc>, TimeParseError> {
        let naive = date.and_hms_opt(12, 0, 0).ok_or(TimeParseError::InvalidDate)?;
        Ok(Utc.from_utc_datetime(&naive))
    }

    pub fn until_datetime(&self) -> Result<DateTime<Utc>, TimeParseError> {
        match self {
            ParsedWhen::Relative(RelativeAmount::Duration(d)) => Ok(Utc::now() + *d),
            ParsedWhen::Relative(RelativeAmount::Months(n)) => Utc::now()
                .checked_add_months(Months::new(*n))
                .ok_or(TimeParseError::InvalidDate),
            ParsedWhen::Absolute(dt) => {
                if *dt <= Utc::now() {
                    return Err(TimeParseError::InThePast);
                }
                Ok(*dt)
            }
        }
    }
}