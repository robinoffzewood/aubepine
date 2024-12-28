use std::collections::HashMap;

use availabilities::Availabilities;
use calendar::{Calendar, Event};
use time::Date;

mod availabilities;
mod calendar;
mod person;

type Name = String;
type AvailabilitiesPerPerson = HashMap<Name, Availabilities>;

#[derive(Debug)]
pub struct CalendarMaker {
    calendar: Calendar,
    availabilities: AvailabilitiesPerPerson,
    persons: HashMap<Name, person::Person>,
}

impl CalendarMaker {
    /// First row contains the month, the year and the days of the week, separated by commas.
    /// The following rows contain the name of the person and the availabilities for each day, each separated by a comma.
    pub fn from_file(filename: &str) -> Self {
        // Use first row to build the calendar
        let file_content = std::fs::read_to_string(filename).expect("Could not read file");
        Self::from_lines(&mut file_content.lines())
    }

    /// Fill the calendar, in order to have one person per day and per event. To find who can be on-call, use the availabilities of each person.
    /// The rules are the following:
    ///  - One person can't be on-call for two consecutive days, except for the Second level on friday, saturday and sunday.
    ///  - One person can't be on-call for two consecutive events, except for the Second level on friday, saturday and sunday.
    ///
    /// Start by filling the First level, day and night, then the Second level, day and night.
    /// Sort the days by the number of available persons, and start by the day with the least available persons.
    /// When finding a person for a day, remove them from the list of available persons for this day, but also the previous and the next day.
    /// Try all the possibilities, and store all the solutions. Each solution is a calendar that is entirely filled with persons.
    /// When all the possibilities have been tried, score each of them, and return the best one.
    /// The score is the sum of events for which the person is an employee, minus the sum of events for which the person is a subcontractor.
    pub fn make_calendar(&mut self, max_subcontractor: u8) {
        'loop_event: for event in [Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly] {
            for subco_nb in 0..=max_subcontractor {
                for availabilities in self.generate_availabilities_with_subco(
                    &self.availabilities.clone(),
                    subco_nb,
                    event,
                ) {
                    let (new_availabilities, new_calendar) =
                        Self::find_next(availabilities, self.calendar.clone(), event);
                    if new_calendar.get_empty_days(&event).is_empty() {
                        self.calendar = new_calendar;
                        self.availabilities = new_availabilities;
                        continue 'loop_event;
                    }
                }
            }
            println!("No solution found for event {:?}", event);
        }
    }

    pub fn print_calendar(&self) {
        for (day, events) in self.calendar.get_all() {
            for event in [Event::FirstDaily,
                Event::FirstNightly,
                Event::SecondDaily,
                Event::SecondNightly] {
                if let Some(name) = events.get(&event) {
                    println!("{}, {:?}, {}", day, event, name);
                }
            }
        }
    }

    fn generate_availabilities_with_subco(
        &mut self,
        input_availabilities: &AvailabilitiesPerPerson,
        subcontractor_to_add: u8,
        event: Event,
    ) -> Vec<AvailabilitiesPerPerson> {
        if subcontractor_to_add == 0 {
            return vec![input_availabilities.to_owned()];
        }

        let subco_name = format!("EXT-{}-{}", event, subcontractor_to_add);
        self.persons.insert(
            subco_name.clone(),
            person::Person::new_subcontractor(subco_name.clone()),
        );

        let mut availabilities_with_subco = vec![input_availabilities.to_owned()];
        for day_ordinal in self.calendar.from().ordinal()..=self.calendar.to().ordinal() {
            let event_str = match event {
                Event::FirstDaily => "1ère SF jour,",
                Event::FirstNightly => "1ère SF nuit,",
                Event::SecondDaily => "2ème SF jour,",
                Event::SecondNightly => "2ème SF nuit,",
            };
            let mut availabilities_str = event_str.to_string();
            for _ in self.calendar.from().ordinal()..day_ordinal {
                availabilities_str.push(',');
            }
            availabilities_str.push('x');
            for _ in day_ordinal..self.calendar.to().ordinal() {
                availabilities_str.push(',');
            }
            let mut extra_availabilities = input_availabilities.clone();
            extra_availabilities
                .entry(subco_name.clone())
                .and_modify(|a| a.merge(self.calendar.from(), &availabilities_str.to_string()))
                .or_insert(Availabilities::from_str(
                    self.calendar.from(),
                    &availabilities_str.to_string(),
                ));
            // availabilities_with_subco.push(extra_availabilities.clone());
            let sub_new_availabilities = self.generate_availabilities_with_subco(
                &extra_availabilities,
                subcontractor_to_add - 1,
                event,
            );
            availabilities_with_subco.extend(sub_new_availabilities);
        }
        availabilities_with_subco
    }

    fn find_next(
        availabilities: AvailabilitiesPerPerson,
        calendar: Calendar,
        event: Event,
    ) -> (AvailabilitiesPerPerson, Calendar) {
        let mut availabilities = availabilities.clone();
        let mut calendar = calendar.clone();
        let remaining_days = calendar.get_empty_days(&event);
        if !remaining_days.is_empty() {
            let day_with_least_availabilities =
                Self::get_day_with_least_availabilities(&availabilities, &remaining_days, event);
            if let Some((day, names)) = day_with_least_availabilities {
                for name in names {
                    let mut new_calendar = calendar.clone();
                    let mut new_availabilities = availabilities.clone();
                    new_calendar.set_for(day, event, name.clone());
                    let her_availabilities = new_availabilities.get_mut(&name).unwrap();
                    Self::update_availabilities(her_availabilities, day, event);
                    (new_availabilities, new_calendar) =
                        Self::find_next(new_availabilities, new_calendar, event);
                    // if there are less empty days than before, consider this branch as successful, and break this loop
                    if new_calendar.get_empty_days(&event).len() < remaining_days.len() {
                        availabilities = new_availabilities;
                        calendar = new_calendar;
                        break;
                    }
                }
            }
        }
        (availabilities, calendar)
    }

    fn get_day_with_least_availabilities(
        availabilities: &AvailabilitiesPerPerson,
        within_days: &Vec<Date>,
        event: Event,
    ) -> Option<(Date, Vec<Name>)> {
        let mut availabilities_per_day = HashMap::new();
        for day in within_days {
            let mut persons = Vec::new();
            for (name, availabilities) in availabilities {
                if availabilities
                    .get(day)
                    .and_then(|a| a.iter().find(|e| *e == &event))
                    .is_some()
                {
                    persons.push(name.to_string());
                }
            }
            availabilities_per_day.insert(day, persons);
        }
        let least = availabilities_per_day
            .iter()
            .min_by_key(|(_, persons)| persons.len())
            .map(|(day, persons)| (day, persons.iter().cloned()))
            .unwrap();
        let day = least.0.to_owned().to_owned();
        let names = availabilities_per_day.get(&day).unwrap().to_owned();
        Some((day, names))
    }

    /// Update the availabilities of a person, given the day and the event that has been requested.
    fn update_availabilities(her_availabilities: &mut Availabilities, day: Date, event: Event) {
        her_availabilities.pop_event(&day, event);
        let is_second_on_the_weekend = (event == Event::SecondDaily
            || event == Event::SecondNightly)
            && (day.weekday() == time::Weekday::Friday
                || day.weekday() == time::Weekday::Saturday
                || day.weekday() == time::Weekday::Sunday);
        let remains_available_next_day = is_second_on_the_weekend
            && (day.weekday() == time::Weekday::Friday || day.weekday() == time::Weekday::Saturday);
        let remains_available_previous_day = is_second_on_the_weekend
            && (day.weekday() == time::Weekday::Saturday || day.weekday() == time::Weekday::Sunday);
        if !remains_available_next_day {
            let next_day = day + time::Duration::days(1);
            her_availabilities.pop_all(&next_day);
        }
        if !remains_available_previous_day {
            let previous_day = day - time::Duration::days(1);
            her_availabilities.pop_all(&previous_day);
        }
        if !remains_available_next_day || !remains_available_previous_day {
            her_availabilities.pop_all(&day);
        }
    }

    fn from_lines(lines: &mut std::str::Lines) -> Self {
        let first_line = lines.next().expect("Empty file!");
        let mut month = None;
        let mut year = None;
        let mut first_day = None;
        let mut last_day = None;
        for (i, token) in first_line.split(",").enumerate() {
            if i == 0 {
                match token.to_ascii_uppercase().as_str() {
                    "JANVIER" => month = Some(time::Month::January),
                    "FEVRIER" => month = Some(time::Month::February),
                    "MARS" => month = Some(time::Month::March),
                    "AVRIL" => month = Some(time::Month::April),
                    "MAI" => month = Some(time::Month::May),
                    "JUIN" => month = Some(time::Month::June),
                    "JUILLET" => month = Some(time::Month::July),
                    "AOUT" => month = Some(time::Month::August),
                    "SEPTEMBRE" => month = Some(time::Month::September),
                    "OCTOBRE" => month = Some(time::Month::October),
                    "NOVEMBRE" => month = Some(time::Month::November),
                    "DECEMBRE" => month = Some(time::Month::December),
                    _ => panic!("Invalid month"),
                }
            } else if i == 1 {
                year = Some(token.parse().expect("Invalid year"));
            } else if i == 2 {
                first_day = Some(token.parse().expect("Invalid day"));
            } else {
                last_day = Some(token.parse().expect("Invalid day"));
            }
        }
        let from =
            Date::from_calendar_date(year.unwrap(), month.unwrap(), first_day.unwrap()).unwrap();
        let to =
            Date::from_calendar_date(year.unwrap(), month.unwrap(), last_day.unwrap()).unwrap();
        let calendar = Calendar::new(from, to);

        let mut availabilities = HashMap::new();
        let mut persons = HashMap::new();
        while let Some(line) = lines.next().as_mut() {
            let (name, availabilities_str) = line.split_once(",").expect("Name missing");
            let name = name.to_string();
            if name.starts_with("EXT") {
                persons.insert(
                    name.clone(),
                    person::Person::new_subcontractor(name.clone()),
                );
            } else {
                persons.insert(name.clone(), person::Person::new_employee(name.clone()));
            }
            availabilities
                .entry(name)
                .and_modify(|a: &mut Availabilities| a.merge(calendar.from(), availabilities_str))
                .or_insert(Availabilities::from_str(
                    calendar.from(),
                    availabilities_str,
                ));
        }

        Self {
            calendar,
            availabilities,
            persons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event::{FirstDaily, FirstNightly};

    #[test]
    fn test_from_lines() {
        let content =
            "JANVIER,2025,1,2,3,4,5\r\nAlice,1ère SF jour,x,x,x,,\r\nAlice,1ère SF nuit,,x,x,,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        assert!(calendar_maker.calendar.from() == Date::from_ordinal_date(2025, 1).unwrap());
        assert!(calendar_maker.calendar.get_all().len() == 5);
        assert!(calendar_maker.persons.contains_key("Alice"));
        assert!(calendar_maker
            .availabilities
            .keys()
            .any(|a| a == "Alice"));
        assert!(
            calendar_maker
                .availabilities
                .get("Alice")
                .unwrap()
                .get(&calendar_maker.calendar.from())
                .unwrap()
                == &vec![FirstDaily]
        );
        assert!(
            calendar_maker
                .availabilities
                .get("Alice")
                .unwrap()
                .get(&Date::from_ordinal_date(2025, 5).unwrap())
                .unwrap()
                == &vec![FirstNightly]
        );
    }

    #[test]
    fn test_get_day_with_least_availabilities() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,x,,\r\nCharlie,1ère SF jour,x,,,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_day_with_least_availabilities(
            &calendar_maker.availabilities,
            &vec![
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        );
        assert_eq!(
            day_with_least_availabilities.unwrap().0,
            Date::from_ordinal_date(2025, 3).unwrap()
        );
    }

    #[test]
    fn test_update_her_availabilities() {
        let wednesday = Date::from_ordinal_date(2025, 1).unwrap();
        let thursday = Date::from_ordinal_date(2025, 2).unwrap();
        let friday = Date::from_ordinal_date(2025, 3).unwrap();
        let saturday = Date::from_ordinal_date(2025, 4).unwrap();
        let sunday = Date::from_ordinal_date(2025, 5).unwrap();

        let content = "JANVIER,2025,1,2,3,4,5\r\nAlice,1ère SF jour,x,x,x,x,x\r\nAlice,2ème SF jour,x,x,x,x,x\r\nAlice,2ème SF nuit,x,x,x,x,x\r\n";
        let mut calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let her_availabilities = calendar_maker.availabilities.get_mut("Alice").unwrap();
        // Get her on call for saturday as SecondDaily. She would still be fully available for sunday and friday, but only as SecondNightly for saturday.
        CalendarMaker::update_availabilities(her_availabilities, saturday, Event::SecondDaily);
        assert_eq!(
            her_availabilities.get(&saturday).unwrap(),
            &vec![Event::SecondNightly]
        );
        assert_eq!(
            her_availabilities.get(&friday).unwrap(),
            &vec![Event::FirstDaily, Event::SecondDaily, Event::SecondNightly]
        );
        assert_eq!(
            her_availabilities.get(&sunday).unwrap(),
            &vec![Event::FirstDaily, Event::SecondDaily, Event::SecondNightly]
        );
        // Get her on call for Thursday as SecondDaily. She would no longer be available for Wednesday and Friday.
        CalendarMaker::update_availabilities(her_availabilities, thursday, Event::SecondDaily);
        assert_eq!(her_availabilities.get(&thursday).unwrap(), &vec![]);
        assert_eq!(her_availabilities.get(&wednesday).unwrap(), &vec![]);
        assert_eq!(her_availabilities.get(&friday).unwrap(), &vec![]);
    }

    #[test]
    fn test_make_calendar_2_persons() {
        let content = "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,x,,x\r\nBob,1ère SF jour,x,x,,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());

        let (_, new_calendar) = CalendarMaker::find_next(
            calendar_maker.availabilities.clone(),
            calendar_maker.calendar.clone(),
            Event::FirstDaily,
        );
        assert!(new_calendar.get_empty_days(&Event::FirstDaily).is_empty()); // all days are filled
        assert!(
            new_calendar.get_for(
                &Date::from_ordinal_date(2025, 1).unwrap(),
                &Event::FirstDaily
            ) == Some(&"Alice".to_string())
        );
        assert!(
            new_calendar.get_for(
                &Date::from_ordinal_date(2025, 2).unwrap(),
                &Event::FirstDaily
            ) == Some(&"Bob".to_string())
        );
        assert!(
            new_calendar.get_for(
                &Date::from_ordinal_date(2025, 3).unwrap(),
                &Event::FirstDaily
            ) == Some(&"Alice".to_string())
        );
    }

    #[test]
    fn test_make_calendar_3_persons() {
        let content = "JANVIER,2025,1,2,3,4,5,6,7\r\nAlice,1ère SF jour,x,x,x,x,,,x\r\nBob,1ère SF jour,,,x,,,x,x\r\nCharlie,1ère SF jour,,x,,,x,x,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());

        let (_, new_calendar) = CalendarMaker::find_next(
            calendar_maker.availabilities.clone(),
            calendar_maker.calendar.clone(),
            Event::FirstDaily,
        );
        assert!(new_calendar.get_empty_days(&Event::FirstDaily).is_empty());
        assert_eq!(
            new_calendar
                .get_all()
                .values()
                .map(|f| f.get(&Event::FirstDaily).unwrap())
                .collect::<Vec<&Name>>(),
            vec!["Alice", "Charlie", "Bob", "Alice", "Charlie", "Bob", "Alice"]
        );
    }

    #[test]
    fn test_generate_availabilities_with_one_subco() {
        let content = "JANVIER,2025,5,6,7\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,x,\r\nCharlie,1ère SF jour,x,,\r\n";
        let mut calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let availabilities = calendar_maker.generate_availabilities_with_subco(
            &calendar_maker.availabilities.clone(),
            1,
            FirstDaily,
        );
        assert_eq!(availabilities.len(), 4);
        for a in availabilities {
            if let Some(days) = a.get("EXT-1D-1") {
                assert_eq!(days.get_all().keys().len(), 3);
            }
        }
    }

    #[test]
    fn test_generate_availabilities_with_two_subco() {
        let content = "JANVIER,2025,5,6,7\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,x,\r\nCharlie,1ère SF jour,x,,\r\n";
        let mut calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let availabilities = calendar_maker.generate_availabilities_with_subco(
            &calendar_maker.availabilities.clone(),
            2,
            FirstDaily,
        );
        // Without subco = 1
        // with one subco = 3
        // with two subco = 3 * 3 = 9
        assert_eq!(availabilities.len(), 13);

        let day_5 = Date::from_ordinal_date(2025, 5).unwrap();
        let day_6 = Date::from_ordinal_date(2025, 6).unwrap();
        let day_7 = Date::from_ordinal_date(2025, 7).unwrap();
        for (i, a) in availabilities.iter().enumerate() {
            if i == 0 {
                assert!(a.get("EXT-1D-1").is_none());
                assert!(a.get("EXT-1D-2").is_none());
            }
            // Check the EXT-2 is there
            if (1..=4).contains(&i) {
                assert!(a
                    .get("EXT-1D-2")
                    .unwrap()
                    .get(&day_5)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if (5..=8).contains(&i) {
                assert!(a
                    .get("EXT-1D-2")
                    .unwrap()
                    .get(&day_6)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if (9..=12).contains(&i) {
                assert!(a
                    .get("EXT-1D-2")
                    .unwrap()
                    .get(&day_7)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            // Check for the EXT-1 absence
            if [1, 5, 9].contains(&i) {
                assert!(a.get("EXT-1D-1").is_none());
            }
            // Check for the EXT-1 presence
            if [2, 6, 10].contains(&i) {
                assert!(a
                    .get("EXT-1D-1")
                    .unwrap()
                    .get(&day_5)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if [3, 7, 11].contains(&i) {
                assert!(a
                    .get("EXT-1D-1")
                    .unwrap()
                    .get(&day_6)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if [4, 8, 12].contains(&i) {
                assert!(a
                    .get("EXT-1D-1")
                    .unwrap()
                    .get(&day_7)
                    .unwrap()
                    .contains(&FirstDaily));
            }
        }
    }
}
