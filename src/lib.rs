use std::collections::HashMap;

use availabilities::Availabilities;
use calendar::{Calendar, Event};
use itertools::Itertools;
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
    _persons: HashMap<Name, person::Person>,
    max_subcontractor: u8,
}

impl CalendarMaker {
    /// First row contains the month, the year and the days of the week, separated by commas.
    /// The following rows contain the name of the person and the availabilities for each day, each separated by a comma.
    pub fn from_file(filename: &str) -> Self {
        // Use first row to build the calendar
        let file_content = std::fs::read_to_string(filename).expect("Could not read file");
        let file_content = file_content
            .strip_prefix("\u{feff}")
            .unwrap_or(&file_content);
        Self::from_lines(&mut file_content.lines())
    }

    /// Fill the calendar, in order to have one person per day and per event. To find who can be on-call, use the availabilities of each person.
    /// The rules are the following:
    ///  - One person can't be on-call for two consecutive days, except for the Second level on friday, saturday and sunday.
    ///  - One person can't be on-call for two consecutive events, except for the Second level on friday, saturday and sunday.
    ///
    /// Start by the days with the least available persons.
    /// When finding a person for a day, remove them from the list of available persons for this day, but also the previous and the next day.
    /// Try all the possibilities, recursively, stopping when all the days are filled.
    pub fn make_calendar(&mut self, max_subcontractor: u8) {
        self.max_subcontractor = max_subcontractor;
        let events = [
            Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly,
        ];
        let all_combinations_of_events = events.iter().permutations(events.len());
        for combination in all_combinations_of_events {
            // println!(
            //     "Trying combination {:?}",
            //     combination
            // );
            let mut solution_found_for_event = Vec::new();
            // Start with a clear calendar and original availabilities
            let mut calendar = self.calendar.clone();
            let mut availabilities = self.availabilities.clone();
            for &event in &combination {
                (calendar, availabilities) = self.make_calendar_for_event(
                    &calendar.clone(),
                    &availabilities.clone(),
                    *event,
                );
                if calendar.get_empty_days(event).is_empty() {
                    solution_found_for_event.push(event);
                } else {
                    // println!(" -> No solution found for event {:?}", event);
                    break;
                }
            }
            if solution_found_for_event.len() == events.len() {
                // println!(" -> All events have a solution!");
                self.calendar = calendar;
                self.availabilities = availabilities;
                break;
            }
        }
    }

    fn make_calendar_for_event(
        &self,
        calendar: &Calendar,
        availabilities: &AvailabilitiesPerPerson,
        event: Event,
    ) -> (Calendar, AvailabilitiesPerPerson) {
        for subco_nb in 0..=self.max_subcontractor {
            for availabilities_with_subco in
                self.generate_availabilities_with_subco(&availabilities.clone(), subco_nb, event)
            {
                let (new_availabilities, new_calendar) =
                    Self::find_next(availabilities_with_subco, calendar.clone(), event);
                if new_calendar.get_empty_days(&event).is_empty() {
                    return (new_calendar, new_availabilities);
                }
            }
        }
        (calendar.clone(), availabilities.clone())
    }

    pub fn print_results(&self) {
        // for (person, availabilities) in &self.availabilities {
        //     println!("{} {}", person, availabilities.format());
        // }
        self.calendar.print();
    }

    fn generate_availabilities_with_subco(
        &self,
        input_availabilities: &AvailabilitiesPerPerson,
        subcontractor_to_add: u8,
        event: Event,
    ) -> Vec<AvailabilitiesPerPerson> {
        if subcontractor_to_add == 0 {
            return vec![input_availabilities.to_owned()];
        }

        let subco_name = format!("EXT-{}", subcontractor_to_add);
        // self.persons.insert(
        //     subco_name.clone(),
        //     person::Person::new_subcontractor(subco_name.clone()),
        // );

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
            // for (person, availabilities) in &availabilities {
            //     println!("{} {}", person, availabilities.format());
            // }
            // calendar.print();
            let days_with_least_availabilities =
                Self::get_days_with_least_availabilities(&availabilities, &remaining_days, event);
            if let Some(days) = days_with_least_availabilities {
                for (day, names) in days {
                    for name in names {
                        let mut new_calendar = calendar.clone();
                        let mut new_availabilities = availabilities.clone();
                        // Set the person for this day, and update her availabilities
                        new_calendar.set_for(day, event, name.clone());
                        let her_availabilities = new_availabilities.get_mut(&name).unwrap();
                        Availabilities::update_availabilities(her_availabilities, day, event);
                        // Continue to find the next person for the next day
                        (new_availabilities, new_calendar) =
                            Self::find_next(new_availabilities, new_calendar, event);
                        // Successful end condition is reached, return the result
                        if new_calendar.get_empty_days(&event).is_empty() {
                            availabilities = new_availabilities;
                            calendar = new_calendar;
                            return (availabilities, calendar);
                        }
                    }
                }
            }
        }
        (availabilities, calendar)
    }

    fn get_days_with_least_availabilities(
        availabilities: &AvailabilitiesPerPerson,
        within_days: &Vec<Date>,
        event: Event,
    ) -> Option<Vec<(Date, Vec<Name>)>> {
        let mut availabilities_per_day = HashMap::new();
        let mut days_per_availabilities = HashMap::new();
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
            let persons_len = persons.len();
            availabilities_per_day.insert(day, persons);
            days_per_availabilities
                .entry(persons_len)
                .and_modify(|d: &mut Vec<&Date>| d.push(day))
                .or_insert(vec![day]);
        }
        let &least = days_per_availabilities.keys().min().expect("No day found");
        if least == 0 {
            return None;
        }
        let mut days_and_names = Vec::new();
        for &day in days_per_availabilities.get(&least).unwrap() {
            let names = availabilities_per_day.get(day).unwrap();
            days_and_names.push((*day, names.to_owned()));
        }
        Some(days_and_names)
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
            _persons: persons,
            max_subcontractor: 0,
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
        assert!(calendar_maker._persons.contains_key("Alice"));
        assert!(calendar_maker.availabilities.keys().any(|a| a == "Alice"));
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
    fn test_get_day_with_least_availabilities_single() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,x,\r\nCharlie,1ère SF jour,x,,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &vec![
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        )
        .unwrap();
        assert_eq!(day_with_least_availabilities.len(), 1);
        assert_eq!(
            day_with_least_availabilities[0].0,
            Date::from_ordinal_date(2025, 3).unwrap()
        );
    }
    #[test]
    fn test_get_day_with_least_availabilities_none() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,,,\r\nBob,1ère SF jour,,,\r\nCharlie,1ère SF jour,,,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &vec![
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        );
        println!("{:?}", day_with_least_availabilities);
        assert!(day_with_least_availabilities.is_none());
    }

    #[test]
    fn test_get_day_with_least_availabilities_dual() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,,\r\nCharlie,1ère SF jour,x,,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &vec![
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        )
        .unwrap();
        assert_eq!(
            day_with_least_availabilities.clone()[0].0,
            Date::from_ordinal_date(2025, 2).unwrap()
        );
        assert_eq!(
            day_with_least_availabilities[1].0,
            Date::from_ordinal_date(2025, 3).unwrap()
        );
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
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
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
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
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
                assert!(a.get("EXT-1").is_none());
                assert!(a.get("EXT-2").is_none());
            }
            // Check the EXT-2 is there
            if (1..=4).contains(&i) {
                assert!(a
                    .get("EXT-2")
                    .unwrap()
                    .get(&day_5)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if (5..=8).contains(&i) {
                assert!(a
                    .get("EXT-2")
                    .unwrap()
                    .get(&day_6)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if (9..=12).contains(&i) {
                assert!(a
                    .get("EXT-2")
                    .unwrap()
                    .get(&day_7)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            // Check for the EXT-1 absence
            if [1, 5, 9].contains(&i) {
                assert!(a.get("EXT-1").is_none());
            }
            // Check for the EXT-1 presence
            if [2, 6, 10].contains(&i) {
                assert!(a
                    .get("EXT-1")
                    .unwrap()
                    .get(&day_5)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if [3, 7, 11].contains(&i) {
                assert!(a
                    .get("EXT-1")
                    .unwrap()
                    .get(&day_6)
                    .unwrap()
                    .contains(&FirstDaily));
            }
            if [4, 8, 12].contains(&i) {
                assert!(a
                    .get("EXT-1")
                    .unwrap()
                    .get(&day_7)
                    .unwrap()
                    .contains(&FirstDaily));
            }
        }
    }
}
