use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Local, LocalResult, NaiveTime, TimeZone, Weekday};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Schedule {
    Interval(Duration),
    Calendar(CalendarSchedule),
}

impl Schedule {
    pub(super) fn next_instant_after(
        &self,
        local_now: DateTime<Local>,
        instant_now: Instant,
    ) -> Instant {
        match self {
            Self::Interval(interval) => instant_now + *interval,
            Self::Calendar(calendar) => {
                let next = calendar.next_after(local_now);
                let wait = (next - local_now)
                    .to_std()
                    .unwrap_or_else(|_| Duration::from_secs(1));
                instant_now + wait.max(Duration::from_secs(1))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CalendarSchedule {
    weekdays: Vec<Weekday>,
    time: NaiveTime,
}

impl CalendarSchedule {
    fn next_after(&self, now: DateTime<Local>) -> DateTime<Local> {
        for days_ahead in 0..=7 {
            let date = now.date_naive() + chrono::Duration::days(days_ahead);
            if !self.weekdays.contains(&date.weekday()) {
                continue;
            }
            let candidate = match Local.from_local_datetime(&date.and_time(self.time)) {
                LocalResult::Single(candidate) => candidate,
                LocalResult::Ambiguous(earlier, _) => earlier,
                LocalResult::None => continue,
            };
            if candidate > now {
                return candidate;
            }
        }

        now + chrono::Duration::days(1)
    }
}

pub(super) fn parse_schedule(schedule: &str) -> Result<Schedule, String> {
    parse_interval(schedule)
        .map(Schedule::Interval)
        .or_else(|_| parse_calendar(schedule).map(Schedule::Calendar))
}

fn parse_interval(schedule: &str) -> Result<Duration, String> {
    let normalized = schedule.trim().to_lowercase();
    let rest = normalized
        .strip_prefix("every ")
        .ok_or_else(|| "expected schedule to start with `every`".to_string())?;
    let mut parts = rest.split_whitespace();
    let first = parts.next().ok_or_else(|| "missing interval".to_string())?;

    let (amount, unit) = match first {
        "second" | "seconds" | "minute" | "minutes" | "hour" | "hours" | "day" | "days" => {
            (1.0, first)
        }
        value => {
            let amount = value
                .parse::<f64>()
                .map_err(|_| "invalid interval amount".to_string())?;
            let unit = parts
                .next()
                .ok_or_else(|| "missing interval unit".to_string())?;
            (amount, unit)
        }
    };

    if amount <= 0.0 {
        return Err("interval must be positive".to_string());
    }

    let seconds = match unit.trim_end_matches('s') {
        "second" => amount,
        "minute" => amount * 60.0,
        "hour" => amount * 60.0 * 60.0,
        "day" => amount * 60.0 * 60.0 * 24.0,
        _ => return Err(format!("unsupported interval unit `{unit}`")),
    };

    Ok(Duration::from_secs(seconds.round().max(1.0) as u64))
}

fn parse_calendar(schedule: &str) -> Result<CalendarSchedule, String> {
    let normalized = schedule.trim().to_lowercase();
    let (days, time) = normalized
        .split_once(" at ")
        .ok_or_else(|| "expected calendar schedule like `friday at 17:00`".to_string())?;
    Ok(CalendarSchedule {
        weekdays: parse_weekdays(days.trim())?,
        time: parse_time(time.trim())?,
    })
}

fn parse_weekdays(days: &str) -> Result<Vec<Weekday>, String> {
    match days {
        "weekday" | "weekdays" => Ok(vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ]),
        "weekend" | "weekends" => Ok(vec![Weekday::Sat, Weekday::Sun]),
        "daily" | "day" | "every day" => Ok(vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ]),
        value => value
            .split(',')
            .map(|day| parse_weekday(day.trim()))
            .collect(),
    }
}

fn parse_weekday(day: &str) -> Result<Weekday, String> {
    match day {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        _ => Err(format!("unsupported weekday `{day}`")),
    }
}

fn parse_time(time: &str) -> Result<NaiveTime, String> {
    let compact = time.replace(' ', "");
    ["%H:%M", "%H:%M:%S", "%-I:%M%P", "%-I%P"]
        .iter()
        .find_map(|format| NaiveTime::parse_from_str(&compact, format).ok())
        .ok_or_else(|| format!("unsupported time `{time}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_existing_interval_schedules() {
        assert_eq!(parse_interval("every minute"), Ok(Duration::from_secs(60)));
        assert_eq!(
            parse_interval("every 1.5 hours"),
            Ok(Duration::from_secs(5400))
        );
    }

    #[test]
    fn parses_named_weekday_calendar_schedule() {
        let schedule = parse_calendar("friday at 17:00").unwrap();

        assert_eq!(schedule.weekdays, vec![Weekday::Fri]);
        assert_eq!(schedule.time, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }

    #[test]
    fn parses_weekdays_with_am_pm_time() {
        let schedule = parse_calendar("weekdays at 9:30am").unwrap();

        assert_eq!(
            schedule.weekdays,
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ]
        );
        assert_eq!(schedule.time, NaiveTime::from_hms_opt(9, 30, 0).unwrap());
    }
}
