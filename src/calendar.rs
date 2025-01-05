//! This class models a calendar, where for each day there are 4 events: 1st daily, 1st nightly, 2nd daily, 2nd nightly.
//! Each event must be associated with a name. That name is the one who is on-call for that event (on-call level + day/night).
//! The calendar is represented as a BTreeMap from a date to a map from an event to a name.
//! The interface with Calendar is made of the following methods:
//! a get_all() method that returns the 4 events and name associated for all the days of the calendar
//! a get_for(day, event) method that returns the name associated with an event and a date
//! a set_for(day, event) method that sets the name associated with a date and an event
//! a get_missing() method that returns the dates and events for which there is no name associated.$

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use time::Date;

use crate::Name;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Event {
    FirstDaily,
    FirstNightly,
    SecondDaily,
    SecondNightly,
}

#[derive(Debug, Clone)]
pub struct Calendar {
    from: Date,
    to: Date,
    days: BTreeMap<Date, HashMap<Event, Name>>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let event_str = match self {
            Event::FirstDaily => "J",
            Event::FirstNightly => "N",
            Event::SecondDaily => "j",
            Event::SecondNightly => "n",
        };
        write!(f, "{}", event_str)
    }
}

impl Calendar {
    pub fn new(from: Date, to: Date) -> Self {
        let mut days = BTreeMap::new();
        for ordinal in from.ordinal()..=to.ordinal() {
            days.insert(
                Date::from_ordinal_date(from.year(), ordinal).ok().unwrap(),
                HashMap::new(),
            );
        }
        Self { from, to, days }
    }

    pub fn from(&self) -> Date {
        self.from
    }

    pub fn to(&self) -> Date {
        self.to
    }

    pub fn get_all(&self) -> &BTreeMap<Date, HashMap<Event, Name>> {
        &self.days
    }

    #[allow(dead_code)] // used in unit tests only
    pub fn get_for(&self, day: &Date, event: &Event) -> Option<&Name> {
        self.days.get(day)?.get(event)
    }

    pub fn set_for(&mut self, day: Date, event: Event, name: Name) {
        self.days
            .entry(day)
            .and_modify(|events| {
                events.insert(event, name.clone());
            })
            .or_insert_with(|| {
                let mut events = HashMap::new();
                events.insert(event, name);
                events
            });
    }

    pub fn get_empty_days(&self, event: &Event) -> Vec<Date> {
        let mut missing = vec![];
        for (day, on_call) in &self.days {
            if !on_call.contains_key(event) {
                missing.push(*day);
            }
        }
        missing
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        let header = format!(
            "     |{}",
            self.days.keys().fold(String::new(), |acc, x| acc
                + &format!("  {:0>2}  |", x.day()))
        );
        s.push_str(format!("{}\r\n", header).as_str());
        // print a line of dashes as long as the line of header
        s.push_str(format!("{}\r\n", "-".repeat(header.len())).as_str());
        for event in &[
            Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly,
        ] {
            s.push_str(format!("{}    |", event).as_str());
            for events in self.days.values() {
                s.push_str(
                    format!(" {:<5}|", events.get(event).unwrap_or(&"   ".to_string())).as_str(),
                );
            }
            s.push_str("\r\n");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Date;

    #[test]
    fn test_calendar_new() {
        let from = Date::from_ordinal_date(2025, 1).unwrap();
        let to = Date::from_ordinal_date(2025, 365).unwrap();
        let calendar = Calendar::new(from, to);
        assert_eq!(calendar.days.len(), 365);
        assert_eq!(calendar.days.get(&from).unwrap().len(), 0);
        assert_eq!(calendar.days.get(&to).unwrap().len(), 0);
    }

    #[test]
    fn test_get() {
        let from = Date::from_ordinal_date(2025, 1).unwrap();
        let to = Date::from_ordinal_date(2025, 10).unwrap();
        let calendar = Calendar::new(from, to);
        assert_eq!(calendar.get_all().len(), 10);
        assert!(calendar.get_for(&from, &Event::FirstDaily).is_none());
    }

    #[test]
    fn test_get_missing() {
        let from = Date::from_ordinal_date(2025, 1).unwrap();
        let to = Date::from_ordinal_date(2025, 10).unwrap();
        let mut calendar = Calendar::new(from, to);
        assert_eq!(calendar.get_empty_days(&Event::FirstDaily).len(), 10);
        calendar.set_for(from, Event::FirstDaily, "Alice".to_string());
        assert_eq!(calendar.get_empty_days(&Event::FirstDaily).len(), 9);
    }
}
