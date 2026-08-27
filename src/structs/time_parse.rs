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
    Combined {
        months: u32,
        duration: Duration,
    },
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
        // changed to unanchor to match specific set 1 component, instead catch without anchor and iter
        let re = Regex::new(r"(\d+)(mo|w|y|s|m|h|d)").unwrap();

        let mut total_duration = Duration::zero();
        let mut total_months: u32 = 0;
        let mut matched_any = false;
        let mut last_end = 0;

        for captures in re.captures_iter(input) {
            // prevents things like "2h abc 30m" if tokens are separated by user input
            let whole = captures.get(0).unwrap();

            if whole.start() != last_end {
                return Ok(None);
            }

            matched_any = true;
            last_end = whole.end();

            let value: i64 = captures[1].parse()?;

            if value <= 0 {
                return Err(TimeParseError::NegativeValue);
            }

            match &captures[2] {
                "s" => {
                    total_duration = total_duration
                        .checked_add(&Duration::seconds(value))
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "m" => {
                    total_duration = total_duration
                        .checked_add(&Duration::minutes(value))
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "h" => {
                    total_duration = total_duration
                        .checked_add(&Duration::hours(value))
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "d" => {
                    total_duration = total_duration
                        .checked_add(&Duration::days(value))
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "w" => {
                    total_duration = total_duration
                        .checked_add(&Duration::weeks(value))
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "mo" => {
                    total_months = total_months
                        .checked_add(value as u32)
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                "y" => {
                    let months = (value as u32)
                        .checked_mul(12)
                        .ok_or(TimeParseError::InvalidDate)?;

                    total_months = total_months
                        .checked_add(months)
                        .ok_or(TimeParseError::InvalidDate)?;
                }
                _ => unreachable!(),
            }
        }

        // Must have matched the entire input.
        if !matched_any || last_end != input.len() {
            return Ok(None);
        }

        // If we only have a duration, keep the existing representation.
        if total_months == 0 {
            return Ok(Some(ParsedWhen::Relative(
                RelativeAmount::Duration(total_duration),
            )));
        }

        // If we have months/years as well as a duration, we need both.
        //
        // This requires extending RelativeAmount to support both calendar
        // months and a fixed duration.
        Ok(Some(ParsedWhen::Relative(
            RelativeAmount::Combined {
                months: total_months,
                duration: total_duration,
            },
        )))
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
            ParsedWhen::Relative(RelativeAmount::Duration(d)) => {
                Utc::now()
                    .checked_add_signed(*d)
                    .ok_or(TimeParseError::InvalidDate)
            }

            ParsedWhen::Relative(RelativeAmount::Months(n)) => {
                Utc::now()
                    .checked_add_months(Months::new(*n))
                    .ok_or(TimeParseError::InvalidDate)
            }

            ParsedWhen::Relative(RelativeAmount::Combined { months, duration }) => {
                let now = Utc::now();

                let after_months = now
                    .checked_add_months(Months::new(*months))
                    .ok_or(TimeParseError::InvalidDate)?;

                after_months
                    .checked_add_signed(*duration)
                    .ok_or(TimeParseError::InvalidDate)
            }

            ParsedWhen::Absolute(dt) => {
                if *dt <= Utc::now() {
                    return Err(TimeParseError::InThePast);
                }

                Ok(*dt)
            }
        }
    }
}