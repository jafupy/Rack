use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Weekday};

#[derive(Clone)]
pub(crate) enum Schedule {
    Interval(chrono::Duration),
    Calendar(CalendarSchedule),
}

#[derive(Clone)]
pub(crate) struct CalendarSchedule {
    days: CalendarDays,
    time: NaiveTime,
}

#[derive(Clone)]
enum CalendarDays {
    Any,
    Weekdays,
    One(Weekday),
}

pub(crate) fn parse_schedule(expression: &str) -> Result<Schedule, String> {
    let normalized = expression.trim().to_lowercase();
    if let Some(rest) = normalized.strip_prefix("every ") {
        return parse_interval(rest).map(Schedule::Interval);
    }

    parse_calendar(&normalized).map(Schedule::Calendar)
}

pub(crate) fn next_after(schedule: &Schedule, after: DateTime<Local>) -> Option<DateTime<Local>> {
    match schedule {
        Schedule::Interval(interval) => Some(after + *interval),
        Schedule::Calendar(calendar) => next_calendar_after(calendar, after),
    }
}

fn parse_interval(expression: &str) -> Result<chrono::Duration, String> {
    let mut parts = expression.split_whitespace();
    let amount = parts
        .next()
        .ok_or_else(|| "missing interval amount".to_string())?
        .parse::<f64>()
        .map_err(|_| "invalid interval amount".to_string())?;
    let unit = parts
        .next()
        .ok_or_else(|| "missing interval unit".to_string())?;

    if amount <= 0.0 {
        return Err("interval must be positive".to_string());
    }

    let seconds = match unit.trim_end_matches('s') {
        "second" => amount,
        "minute" => amount * 60.0,
        "hour" => amount * 60.0 * 60.0,
        "day" => amount * 60.0 * 60.0 * 24.0,
        _ => return Err(format!("unsupported interval unit '{unit}'")),
    };

    let seconds = seconds.round() as i64;
    if seconds < 1 {
        return Err("interval must be at least one second".to_string());
    }

    Ok(chrono::Duration::seconds(seconds))
}

fn parse_calendar(expression: &str) -> Result<CalendarSchedule, String> {
    let (days, time) = if let Some((day, time)) = expression.split_once(" at ") {
        (parse_days(day.trim())?, parse_time(time.trim())?)
    } else {
        (CalendarDays::Any, parse_time(expression)?)
    };

    Ok(CalendarSchedule { days, time })
}

fn parse_days(value: &str) -> Result<CalendarDays, String> {
    match value {
        "weekday" | "weekdays" => Ok(CalendarDays::Weekdays),
        "monday" | "mon" => Ok(CalendarDays::One(Weekday::Mon)),
        "tuesday" | "tue" | "tues" => Ok(CalendarDays::One(Weekday::Tue)),
        "wednesday" | "wed" => Ok(CalendarDays::One(Weekday::Wed)),
        "thursday" | "thu" | "thur" | "thurs" => Ok(CalendarDays::One(Weekday::Thu)),
        "friday" | "fri" => Ok(CalendarDays::One(Weekday::Fri)),
        "saturday" | "sat" => Ok(CalendarDays::One(Weekday::Sat)),
        "sunday" | "sun" => Ok(CalendarDays::One(Weekday::Sun)),
        _ => Err(format!("unsupported calendar day '{value}'")),
    }
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    let compact = value.replace(' ', "");
    let (clock, suffix) = if let Some(clock) = compact.strip_suffix("am") {
        (clock, Some("am"))
    } else if let Some(clock) = compact.strip_suffix("pm") {
        (clock, Some("pm"))
    } else {
        (compact.as_str(), None)
    };

    let mut pieces = clock.split(':');
    let mut hour = pieces
        .next()
        .ok_or_else(|| "missing hour".to_string())?
        .parse::<u32>()
        .map_err(|_| "invalid hour".to_string())?;
    let minute = pieces
        .next()
        .map(|minute| {
            minute
                .parse::<u32>()
                .map_err(|_| "invalid minute".to_string())
        })
        .transpose()?
        .unwrap_or(0);

    match suffix {
        Some("am") if hour == 12 => hour = 0,
        Some("pm") if hour < 12 => hour += 12,
        _ => {}
    }

    NaiveTime::from_hms_opt(hour, minute, 0).ok_or_else(|| format!("invalid time '{value}'"))
}

fn next_calendar_after(
    schedule: &CalendarSchedule,
    after: DateTime<Local>,
) -> Option<DateTime<Local>> {
    for offset in 0..14 {
        let date = after.date_naive() + chrono::Duration::days(offset);
        let weekday = date.weekday();
        let day_matches = match schedule.days {
            CalendarDays::Any => true,
            CalendarDays::Weekdays => !matches!(weekday, Weekday::Sat | Weekday::Sun),
            CalendarDays::One(day) => weekday == day,
        };
        if !day_matches {
            continue;
        }

        let naive = date.and_time(schedule.time);
        let Some(candidate) = Local.from_local_datetime(&naive).single() else {
            continue;
        };
        if candidate > after {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interval_schedules() {
        assert!(matches!(
            parse_schedule("every 5 minutes").unwrap(),
            Schedule::Interval(_)
        ));
        assert!(parse_schedule("every 0 seconds").is_err());
    }

    #[test]
    fn parses_calendar_schedules() {
        assert!(matches!(
            parse_schedule("weekdays at 9:30am").unwrap(),
            Schedule::Calendar(_)
        ));
        assert!(matches!(
            parse_schedule("friday at 17:00").unwrap(),
            Schedule::Calendar(_)
        ));
        assert!(parse_schedule("noday at 9am").is_err());
    }
}
