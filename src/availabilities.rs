use std::collections::HashMap;

use itertools::Itertools;
use time::Date;

use crate::calendar::Event;

#[derive(Debug, Clone)]
pub struct Availabilities {
    days: HashMap<Date, Vec<Event>>,
}

impl Availabilities {
    /// Input must contain the name of the person, the level of on-call, and the availabilities, each separated by a comma.
    /// The valid availabilities are 'x' or 'X'.
    pub fn from_str(from: Date, line: &str) -> Self {
        Self {
            days: Self::map_from_str(from, line),
        }
    }

    pub fn merge(&mut self, from: Date, line: &str) {
        let new_map = Self::map_from_str(from, line);
        for (day, availabilities) in new_map {
            self.days
                .entry(day)
                .and_modify(|v| v.extend(availabilities.clone()))
                .or_insert(availabilities);
        }
    }

    pub fn get(&self, day: &Date) -> Option<&Vec<Event>> {
        self.days.get(day)
    }

    #[allow(dead_code)]
    pub fn get_all(&self) -> &HashMap<Date, Vec<Event>> {
        &self.days
    }

    pub fn pop_all(&mut self, day: &Date) {
        if let Some(availabilities) = self.days.get_mut(day) {
            availabilities.clear();
        }
    }

    pub fn pop_event(&mut self, day: &Date, event: Event) -> Option<Event> {
        let availabilities = self.days.get_mut(day)?;
        let popped = availabilities
            .iter()
            .position(|a| *a == event)
            .map(|i| availabilities.remove(i));
        popped
    }

    fn map_from_str(from: Date, line: &str) -> HashMap<Date, Vec<Event>> {
        let mut days = HashMap::new();
        let mut day = from;
        let (level_str, availabilities_str) = line.split_once(",").unwrap();
        let level = match level_str {
            "1ère SF jour" => Event::FirstDaily,
            "1ère SF nuit" => Event::FirstNightly,
            "2ème SF jour" => Event::SecondDaily,
            "2ème SF nuit" => Event::SecondNightly,
            _ => panic!(
                "Unknown on-call level. Must be within (1ère SF jour..2ème SF nuit): {}",
                level_str
            ),
        };
        for token in availabilities_str.split(",") {
            if token.is_empty() {
                days.insert(day, vec![]);
            } else {
                days.entry(day)
                    .and_modify(|v: &mut Vec<Event>| v.push(level))
                    .or_insert(vec![level]);
            }
            day = day.next_day().unwrap();
        }
        days
    }

    /// Update the availabilities of a person, given the day and the event that has been requested.
    pub fn update_availabilities(her_availabilities: &mut Availabilities, day: Date, event: Event) {
        let next_day = day + time::Duration::days(1);
        let previous_day = day - time::Duration::days(1);
        her_availabilities.pop_event(&day, event);
        let is_second_on_the_weekend = (event == Event::SecondDaily
            || event == Event::SecondNightly)
            && (day.weekday() == time::Weekday::Friday
                || day.weekday() == time::Weekday::Saturday
                || day.weekday() == time::Weekday::Sunday);
        if !is_second_on_the_weekend {
            her_availabilities.pop_all(&day);
            her_availabilities.pop_all(&previous_day);
            her_availabilities.pop_all(&next_day);
        } else {
            her_availabilities.pop_event(&day, Event::FirstDaily);
            her_availabilities.pop_event(&day, Event::FirstNightly);
        }

        let remains_available_as_second_next_day = is_second_on_the_weekend
            && (day.weekday() == time::Weekday::Friday || day.weekday() == time::Weekday::Saturday);
        if remains_available_as_second_next_day {
            her_availabilities.pop_event(&next_day, Event::FirstDaily);
            her_availabilities.pop_event(&next_day, Event::FirstNightly);
        } else {
            her_availabilities.pop_all(&next_day);
        }

        let remains_available_as_second_previous_day = is_second_on_the_weekend
            && (day.weekday() == time::Weekday::Saturday || day.weekday() == time::Weekday::Sunday);
        if remains_available_as_second_previous_day {
            her_availabilities.pop_event(&previous_day, Event::FirstDaily);
            her_availabilities.pop_event(&previous_day, Event::FirstNightly);
        } else {
            her_availabilities.pop_all(&previous_day);
        }
    }

    #[allow(dead_code)]
    pub fn format(&self) -> String {
        // For each day, print a line with a letter corresponding to the availability, and a space otherwise.
        let mut formatted = String::new();
        for day_ordinal in self.days.keys().sorted() {
            let availabilities = self.days.get(day_ordinal).unwrap();
            formatted.push_str(" | ");
            for event in &[
                Event::FirstDaily,
                Event::FirstNightly,
                Event::SecondDaily,
                Event::SecondNightly,
            ] {
                if availabilities.contains(event) {
                    let code = match event {
                        Event::FirstDaily => 'J',
                        Event::FirstNightly => 'N',
                        Event::SecondDaily => 'j',
                        Event::SecondNightly => 'n',
                    };
                    formatted.push(code);
                } else {
                    formatted.push(' ');
                };
            }
        }
        formatted.push_str(" |");
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_availabilities() {
        let day_1 = Date::from_ordinal_date(2025, 1).unwrap();
        let str_1j = "1ère SF jour,x,,,,,,,X,";
        let str_1n = "1ère SF nuit,,x,,,,,,,x";
        let str_2j = "2ème SF jour,,,,x,,,,,x";
        let str_2n = "2ème SF nuit,,,,,x,,,x,";
        let mut availabilities = Availabilities::from_str(day_1, str_1j);
        availabilities.merge(day_1, str_1n);
        availabilities.merge(day_1, str_2j);
        availabilities.merge(day_1, str_2n);
        assert_eq!(availabilities.days.len(), 9);
        // 1D
        let mut day = day_1;
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::FirstDaily])
        );
        // 1N
        day = day.next_day().unwrap();
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::FirstNightly])
        );
        // No Event
        day = day.next_day().unwrap();
        assert_eq!(availabilities.days.get(&day), Some(&vec![]));
        // 2D
        day = day.next_day().unwrap();
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::SecondDaily])
        );
        // 2N
        day = day.next_day().unwrap();
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::SecondNightly])
        );
        // No Event
        day = day.next_day().unwrap();
        assert_eq!(availabilities.days.get(&day), Some(&vec![]));
        // No Event
        day = day.next_day().unwrap();
        assert_eq!(availabilities.days.get(&day), Some(&vec![]));
        // 1D-2N
        day = day.next_day().unwrap();
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::FirstDaily, Event::SecondNightly])
        );
        // 2D-1N
        day = day.next_day().unwrap();
        assert_eq!(
            availabilities.days.get(&day),
            Some(&vec![Event::FirstNightly, Event::SecondDaily])
        );
    }

    #[test]
    fn test_pop_single_event() {
        let day_1 = Date::from_ordinal_date(2025, 1).unwrap();
        let str_1j = "1ère SF jour,x,,,,,,,X,";

        let mut availabilities = Availabilities::from_str(day_1, str_1j);
        let a = availabilities.pop_event(&day_1, Event::FirstDaily);
        assert_eq!(a, Some(Event::FirstDaily));
        assert_eq!(availabilities.days.get(&day_1), Some(&vec![]));
        let a = availabilities.pop_event(&day_1, Event::FirstDaily);
        assert_eq!(a, None);
    }

    #[test]
    fn test_pop_dual_event() {
        let day_1 = Date::from_ordinal_date(2025, 1).unwrap();
        let str_1j = "1ère SF jour,x,,,,,,,X,";
        let str_1n = "1ère SF nuit,,,,,,,,,";
        let str_2j = "2ème SF jour,x,,,,,,,X,";
        let str_2n = "2ème SF nuit,x,,,,,,,X,";
        let mut availabilities = Availabilities::from_str(day_1, str_1j);
        availabilities.merge(day_1, str_1n);
        availabilities.merge(day_1, str_2j);
        availabilities.merge(day_1, str_2n);

        let a = availabilities.pop_event(&day_1, Event::FirstDaily);
        assert_eq!(a, Some(Event::FirstDaily));
    }

    #[test]
    fn test_update_her_availabilities() {
        let wednesday = Date::from_ordinal_date(2025, 1).unwrap();
        let thursday = Date::from_ordinal_date(2025, 2).unwrap();
        let friday = Date::from_ordinal_date(2025, 3).unwrap();
        let saturday = Date::from_ordinal_date(2025, 4).unwrap();
        let sunday = Date::from_ordinal_date(2025, 5).unwrap();

        let str_1j = "1ère SF jour,x,x,x,x,x";
        let str_1n = "1ère SF nuit,x,x,x,x,x";
        let str_2j = "2ème SF jour,x,x,x,x,x";
        let str_2n = "2ème SF nuit,x,x,x,x,x";

        let mut availabilities = Availabilities::from_str(wednesday, str_1j);
        availabilities.merge(wednesday, str_1n);
        availabilities.merge(wednesday, str_2j);
        availabilities.merge(wednesday, str_2n);
        let all = vec![
            Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly,
        ];
        let second = vec![Event::SecondDaily, Event::SecondNightly];

        let mut av_cloned = availabilities.clone();
        // Get her on call for Wednesday as FirstDaily. She would no longer be available for Thursday.
        Availabilities::update_availabilities(&mut av_cloned, wednesday, Event::FirstDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &all);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &all);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Thursday as FirstDaily. She would no longer be available for Wednesday and Friday.
        Availabilities::update_availabilities(&mut av_cloned, thursday, Event::FirstDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &all);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Friday as FirstDaily. She would no longer be available for Thursday and Saturday.
        Availabilities::update_availabilities(&mut av_cloned, friday, Event::FirstDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Saturday as FirstDaily. She would no longer be available for Friday and Sunday.
        Availabilities::update_availabilities(&mut av_cloned, saturday, Event::FirstDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &all);
        assert_eq!(av_cloned.get(&friday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &vec![]);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Sunday as FirstDaily. She would no longer be available for Saturday.
        Availabilities::update_availabilities(&mut av_cloned, sunday, Event::FirstDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &all);
        assert_eq!(av_cloned.get(&friday).unwrap(), &all);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &vec![]);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Wednesday as SecondDaily. She would no longer be available for Thursday.
        Availabilities::update_availabilities(&mut av_cloned, wednesday, Event::SecondDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &all);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &all);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Thursday as SecondDaily. She would no longer be available for Wednesday and Friday.
        Availabilities::update_availabilities(&mut av_cloned, thursday, Event::SecondDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &all);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Friday as SecondDaily. She would no longer be available for Thursday but Saturday for SecondDaily and SecondNightly.
        Availabilities::update_availabilities(&mut av_cloned, friday, Event::SecondDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &vec![]);
        assert_eq!(av_cloned.get(&friday).unwrap(), &vec![Event::SecondNightly]);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &second);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &all);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Saturday as SecondDaily. She would no longer be available for Friday and Sunday as First, but Second.
        Availabilities::update_availabilities(&mut av_cloned, saturday, Event::SecondDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &all);
        assert_eq!(av_cloned.get(&friday).unwrap(), &second);
        assert_eq!(
            av_cloned.get(&saturday).unwrap(),
            &vec![Event::SecondNightly]
        );
        assert_eq!(av_cloned.get(&sunday).unwrap(), &second);

        let mut av_cloned = availabilities.clone();
        // Get her on call for Sunday as SecondDaily. She would no longer be available for Saturday.
        Availabilities::update_availabilities(&mut av_cloned, sunday, Event::SecondDaily);
        assert_eq!(av_cloned.get(&wednesday).unwrap(), &all);
        assert_eq!(av_cloned.get(&thursday).unwrap(), &all);
        assert_eq!(av_cloned.get(&friday).unwrap(), &all);
        assert_eq!(av_cloned.get(&saturday).unwrap(), &second);
        assert_eq!(av_cloned.get(&sunday).unwrap(), &vec![Event::SecondNightly]);
    }
}
