use std::collections::{BTreeMap, HashMap};

use availabilities::Availabilities;
use calendar::{Calendar, Event};
use itertools::Itertools;
use time::Date;

mod availabilities;
mod calendar;

type Name = String;
type AvailabilitiesPerPerson = HashMap<Name, Availabilities>;
type ProblematicDays = BTreeMap<(Date, Event), u8>;

#[derive(Debug)]
pub struct CalendarMaker {
    calendar: Calendar,
    availabilities: AvailabilitiesPerPerson,
    problematic_days: ProblematicDays,
    max_subcontractor: u8,
    verbose: bool,
}

impl CalendarMaker {
    /// First row contains the month, the year and the days of the week, separated by commas.
    /// The following rows contain the name of the person and the availabilities for each day, each separated by a comma.
    pub fn from_file(filename: &str) -> Self {
        let mut calendar_maker;
        // Use first row to build the calendar
        let file_content = std::fs::read_to_string(filename).expect("Could not read file");
        let file_content = file_content
            .strip_prefix("\u{feff}")
            .unwrap_or(&file_content);
        calendar_maker = Self::from_lines(&mut file_content.lines());
        calendar_maker.take_initial_allocations(file_content.lines());
        calendar_maker
    }

    /// Fill the calendar, in order to have one person per day and per event. To find who can be on-call, use the availabilities of each person.
    /// The rules are the following:
    ///  - One person can't be on-call for two consecutive days, except for the Second level on friday, saturday and sunday.
    ///  - One person can't be on-call for two consecutive events, except for the Second level on friday, saturday and sunday.
    ///
    /// Start by the days with the least available persons.
    /// When finding a person for a day, remove them from the list of available persons for this day, but also the previous and the next day.
    /// Try all the possibilities, recursively, stopping when all the days are filled.
    /// Try first without adding extra ressources, then add one subcontractor, then two, etc. up to the maximum number of subcontractors passed as argument.
    pub fn make_calendar(&mut self, max_subcontractor: u8, verbose: bool) {
        self.max_subcontractor = max_subcontractor;
        self.verbose = verbose;
        for i in 0..=max_subcontractor {
            if self.verbose {
                println!("Trying with {} subcontractor(s)", i);
            }
            match self.try_all_permutations() {
                Err(problematic_days) => {
                    if let Some(most_problematic_day) = problematic_days.iter().max_by_key(|e| e.1)
                    {
                        println!(
                            "Most problematic day / event : {:?} / {:?} ({})",
                            most_problematic_day.0 .0,
                            most_problematic_day.0 .1,
                            most_problematic_day.1
                        );
                    }
                    self.problematic_days = problematic_days.clone();
                    let most_problematic_day_and_event =
                        problematic_days.iter().max_by_key(|e| e.1).unwrap().0;
                    let subco_name = format!("EXT-{}", i);
                    let new_availabilities = self.add_subco_for_this_day_and_event(
                        &self.availabilities.clone(),
                        &subco_name,
                        most_problematic_day_and_event.0.ordinal(),
                        most_problematic_day_and_event.1,
                    );
                    self.availabilities = new_availabilities;
                    continue;
                }
                Ok((cal, av)) => {
                    self.calendar = cal;
                    self.availabilities = av;
                    break;
                }
            }
        }
    }

    fn take_initial_allocations(&mut self, lines: std::str::Lines) {
        // Skip the first line, it's the header
        let lines = lines.skip(1);
        for line in lines {
            let (name, availabilities_str) = line.split_once([',', ';']).expect("Name missing");
            let on_call_allocations =
                Availabilities::parse_initial_allocations(self.calendar.from(), availabilities_str);
            for (day, event) in on_call_allocations {
                self.calendar.set_for(day, event, name.to_string());
                let her_availabilities = self.availabilities.get_mut(name).unwrap();
                Availabilities::update_availabilities(her_availabilities, day, event);
            }
        }
    }

    /// Try all the permutations of the events, and return the first solution found.
    fn try_all_permutations(&self) -> Result<(Calendar, AvailabilitiesPerPerson), ProblematicDays> {
        let events = [
            Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly,
        ];
        let mut problematic_days = ProblematicDays::new();
        let all_permutations_of_events = events.iter().permutations(events.len());
        for permutation in all_permutations_of_events {
            if self.verbose {
                println!("Trying permutation {:?}", permutation);
            }
            let mut solution_found_for_event = Vec::new();
            // Start with a clear calendar and original availabilities
            let mut calendar = self.calendar.clone();
            let mut availabilities = self.availabilities.clone();
            let mut problematic_day;
            for &event in &permutation {
                (calendar, availabilities, problematic_day) = self.make_calendar_for_event(
                    &calendar.clone(),
                    &availabilities.clone(),
                    *event,
                );
                if calendar.get_empty_days(event).is_empty() {
                    solution_found_for_event.push(event);
                } else {
                    if self.verbose {
                        println!(" -> No solution found for event {:?}", event);
                    }
                    if let Some(problematic_day) = problematic_day {
                        problematic_days
                            .entry((problematic_day, *event))
                            .and_modify(|v| *v += 1)
                            .or_insert(0);
                    }
                    break;
                }
            }
            if solution_found_for_event.len() == events.len() {
                return Ok((calendar, availabilities));
            }
        }
        Err(problematic_days)
    }

    fn make_calendar_for_event(
        &self,
        calendar: &Calendar,
        availabilities: &AvailabilitiesPerPerson,
        event: Event,
    ) -> (Calendar, AvailabilitiesPerPerson, Option<Date>) {
        let (new_availabilities, new_calendar, problematic_day, _) =
            Self::find_next(availabilities.clone(), calendar.clone(), event, 0);
        if new_calendar.get_empty_days(&event).is_empty() {
            return (new_calendar, new_availabilities, None);
        }
        (calendar.clone(), availabilities.clone(), problematic_day)
    }

    pub fn calendar_as_string(&self) -> String {
        self.calendar.to_string()
    }

    /// Add a subcontractor for the day and event passed in argument.
    fn add_subco_for_this_day_and_event(
        &self,
        availabilities: &HashMap<String, Availabilities>,
        subco_name: &str,
        day_ordinal: u16,
        event: Event,
    ) -> AvailabilitiesPerPerson {
        let event_str = match event {
            Event::FirstDaily => "1ère SF jour",
            Event::FirstNightly => "1ère SF nuit",
            Event::SecondDaily => "2ème SF jour",
            Event::SecondNightly => "2ème SF nuit",
        };
        let mut availabilities_str = event_str.to_string();
        for _ in self.calendar.from().ordinal()..=day_ordinal - 1 {
            availabilities_str.push_str(",x");
        }
        availabilities_str.push(',');
        for _ in day_ordinal + 1..=self.calendar.to().ordinal() {
            availabilities_str.push_str(",x");
        }
        let mut new_availabilities = availabilities.clone();
        new_availabilities
            .entry(subco_name.to_owned())
            .and_modify(|a| a.merge(self.calendar.from(), &availabilities_str.to_string()))
            .or_insert(Availabilities::from_str(
                self.calendar.from(),
                &availabilities_str.to_string(),
            ));
        new_availabilities
    }

    /// Recursive function to find the next person for the next empty day
    fn find_next(
        availabilities: AvailabilitiesPerPerson,
        calendar: Calendar,
        event: Event,
        recursion_depth: u16,
    ) -> (AvailabilitiesPerPerson, Calendar, Option<Date>, u16) {
        let availabilities = availabilities.clone();
        let calendar = calendar.clone();
        let mut problematic_day = None;
        let remaining_days = calendar.get_empty_days(&event);
        if !remaining_days.is_empty() {
            let days_and_names =
                Self::get_days_with_least_availabilities(&availabilities, &remaining_days, event);
            // Check for premature stop, if there's 2 consecutive days with only the same person available
            if Self::check_for_premature_stop(&days_and_names, &event) {
                return (
                    availabilities,
                    calendar,
                    problematic_day,
                    recursion_depth + 1,
                );
            }
            let mut all_permutations_of_days =
                days_and_names.iter().permutations(days_and_names.len());
            for (day, names) in all_permutations_of_days.next().unwrap() {
                problematic_day = Some(*day);
                if names.is_empty() {
                    // No more possibilities, return the current state
                    return (
                        availabilities,
                        calendar,
                        problematic_day,
                        recursion_depth + 1,
                    );
                }
                // println!(
                //     "Recursion depth: {}, Event: {:?}, Day: {}, Names: {:?}",
                //     recursion_depth, event, day, names
                // );
                let sorted_by_least_on_call = Self::sort_names_by_least_on_call(names, &calendar);
                let mut all_permutations_of_names = sorted_by_least_on_call
                    .iter()
                    .permutations(sorted_by_least_on_call.len());
                for name in all_permutations_of_names.next().unwrap() {
                    let mut new_calendar = calendar.clone();
                    let mut new_availabilities = availabilities.clone();
                    let new_recursion_depth;
                    // Set the person for this day, and update her availabilities
                    new_calendar.set_for(*day, event, name.clone());
                    let her_availabilities = new_availabilities.get_mut(name).unwrap();
                    Availabilities::update_availabilities(her_availabilities, *day, event);
                    // Continue to find the next person for the next day
                    (
                        new_availabilities,
                        new_calendar,
                        problematic_day,
                        new_recursion_depth,
                    ) = Self::find_next(
                        new_availabilities,
                        new_calendar,
                        event,
                        recursion_depth + 1,
                    );
                    // Successful end condition is reached, return the result
                    if new_calendar.get_empty_days(&event).is_empty() {
                        return (new_availabilities, new_calendar, None, new_recursion_depth);
                    }
                }
            }
        }
        (availabilities, calendar, problematic_day, recursion_depth)
    }

    /// Sort the names by the least on-call days, allow to balance the on-call days between all the persons
    fn sort_names_by_least_on_call(names: &[Name], calendar: &Calendar) -> Vec<Name> {
        let mut names_and_count = HashMap::new();
        for name in names.iter() {
            let count = calendar
                .get_all()
                .values()
                .filter(|f| Self::is_on_call(f, name))
                .count();
            names_and_count.insert(name, count);
        }
        let sorted_names = names
            .iter()
            .sorted_by_key(|n| names_and_count.get(n).unwrap())
            .cloned()
            .collect();
        sorted_names
    }

    /// Return true if the person designated by `name` is on call in one of the event passed in argument `availabilities`
    fn is_on_call(availabilities: &HashMap<Event, Name>, name: &Name) -> bool {
        for event in [
            Event::FirstDaily,
            Event::FirstNightly,
            Event::SecondDaily,
            Event::SecondNightly,
        ] {
            if let Some(on_call) = availabilities.get(&event) {
                if name == on_call {
                    return true;
                }
            }
        }
        false
    }

    /// Return true if there's 2 consecutive week days with only the same person available
    fn check_for_premature_stop(days_and_names: &[(Date, Vec<Name>)], event: &Event) -> bool {
        if days_and_names.len() < 2 {
            return false;
        }
        for i in 0..days_and_names.len() - 1 {
            // continue if there's more than one person available
            if days_and_names[i].1.len() != 1 {
                continue;
            }
            // Continue if one of the day is a week-end, and we're searching a person available for a Second level event
            let is_second_level = event == &Event::SecondDaily || event == &Event::SecondNightly;
            let one_of_the_day_is_weekend =
                Self::is_weekend(days_and_names[i].0) || Self::is_weekend(days_and_names[i + 1].0);
            if one_of_the_day_is_weekend && is_second_level {
                continue;
            }
            // Return true if there's 2 consecutive days with only the same person available
            let are_consecutive_days = days_and_names[i]
                .0
                .ordinal()
                .abs_diff(days_and_names[i + 1].0.ordinal())
                == 1;
            let is_same_person = days_and_names[i].1 == days_and_names[i + 1].1;
            if are_consecutive_days && is_same_person {
                return true;
            }
        }
        false
    }

    /// Returns true if the day is in the week-end (saturday or sunday)
    fn is_weekend(day: Date) -> bool {
        day.weekday() == time::Weekday::Saturday || day.weekday() == time::Weekday::Sunday
    }

    /// Return the days with the least availabilities for the event passed in argument
    fn get_days_with_least_availabilities(
        availabilities: &AvailabilitiesPerPerson,
        within_days: &[Date],
        event: Event,
    ) -> Vec<(Date, Vec<Name>)> {
        let mut availabilities_per_day = HashMap::new();
        let mut days_per_availabilities = HashMap::new();
        // Sorting the days allow to have a deterministic result
        for day in within_days.iter().sorted() {
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
        let mut days_and_names = Vec::new();
        for &day in days_per_availabilities.get(&least).unwrap() {
            let names = availabilities_per_day.get(day).unwrap();
            // Sorting the names allow to have a deterministic result
            let sorted_names = names.iter().cloned().sorted().collect();
            days_and_names.push((*day, sorted_names));
        }
        days_and_names
    }

    fn from_lines(lines: &mut std::str::Lines) -> Self {
        let first_line = lines.next().expect("Empty file!");
        let mut month = None;
        let mut year = None;
        let mut first_day = None;
        let mut last_day = None;
        for (i, token) in first_line.split(&[',', ';']).enumerate() {
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
        while let Some(line) = lines.next().as_mut() {
            let (name, availabilities_str) = line.split_once([',', ';']).expect("Name missing");
            availabilities
                .entry(name.to_string())
                .and_modify(|a: &mut Availabilities| a.merge(calendar.from(), availabilities_str))
                .or_insert(Availabilities::from_str(
                    calendar.from(),
                    availabilities_str,
                ));
        }

        Self {
            calendar,
            availabilities,
            problematic_days: BTreeMap::new(),
            max_subcontractor: 0,
            verbose: false,
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
            "JANVIER,2025,1,2,3,4,5\r\nAlice,1ère SF jour,,,,x,x\r\nAlice,1ère SF nuit,x,,,x,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        assert!(calendar_maker.calendar.from() == Date::from_ordinal_date(2025, 1).unwrap());
        assert!(calendar_maker.calendar.get_all().len() == 5);
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
    fn test_take_initial_allocations() {
        let content =
            "JANVIER,2025,1,2,3,4,5\r\nAlice,1ère SF jour,,1,,x,x\r\nAlice,1ère SF nuit,x,,,x,\r\n";
        let mut calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        calendar_maker.take_initial_allocations(content.lines());
        assert!(
            calendar_maker
                .calendar
                .get_for(&Date::from_ordinal_date(2025, 2).unwrap(), &FirstDaily)
                == Some(&"Alice".to_string())
        );
        // Because she's already on call the 2nd day, she's not available anymore the 1st and 3rd day
        assert!(
            calendar_maker
                .availabilities
                .get("Alice")
                .unwrap()
                .get(&calendar_maker.calendar.from())
                .unwrap()
                == &vec![]
        );
        assert!(
            calendar_maker
                .availabilities
                .get("Alice")
                .unwrap()
                .get(&Date::from_ordinal_date(2025, 3).unwrap())
                .unwrap()
                == &vec![]
        );
    }

    #[test]
    fn test_get_day_with_least_availabilities_single() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,,,\r\nBob,1ère SF jour,,,x\r\nCharlie,1ère SF jour,,x,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &[
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        );
        assert_eq!(day_with_least_availabilities.len(), 1);
        assert_eq!(
            day_with_least_availabilities[0].0,
            Date::from_ordinal_date(2025, 3).unwrap()
        );
    }
    #[test]
    fn test_get_day_with_least_availabilities_none() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,x,x,x\r\nBob,1ère SF jour,x,x,x\r\nCharlie,1ère SF jour,x,x,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &[
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        );
        println!("{:?}", day_with_least_availabilities);
        assert!(day_with_least_availabilities.first().unwrap().1.is_empty());
    }

    #[test]
    fn test_get_day_with_least_availabilities_dual() {
        let content =
            "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,,,\r\nBob,1ère SF jour,,x,x\r\nCharlie,1ère SF jour,,x,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());
        let day_with_least_availabilities = CalendarMaker::get_days_with_least_availabilities(
            &calendar_maker.availabilities,
            &[
                Date::from_ordinal_date(2025, 1).unwrap(),
                Date::from_ordinal_date(2025, 2).unwrap(),
                Date::from_ordinal_date(2025, 3).unwrap(),
            ],
            FirstDaily,
        );
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
        let content = "JANVIER,2025,1,2,3\r\nAlice,1ère SF jour,,x,\r\nBob,1ère SF jour,,,x,\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());

        let (_, new_calendar, _, _) = CalendarMaker::find_next(
            calendar_maker.availabilities.clone(),
            calendar_maker.calendar.clone(),
            Event::FirstDaily,
            0,
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
        let content = "JANVIER,2025,1,2,3,4,5,6,7\r\nAlice,1ère SF jour,,,,,x,x,\r\nBob,1ère SF jour,x,x,,x,x,,\r\nCharlie,1ère SF jour,x,,x,x,,,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());

        let (_, new_calendar, _, _) = CalendarMaker::find_next(
            calendar_maker.availabilities.clone(),
            calendar_maker.calendar.clone(),
            Event::FirstDaily,
            0,
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
    fn test_sort_names_by_least_on_call() {
        let content = "JANVIER,2025,1,2,3,4,5,6,7\r\nAlice,1ère SF jour,,,,,x,x,\r\nBob,1ère SF jour,x,x,,x,x,,\r\nCharlie,1ère SF jour,x,,x,x,,,x\r\n";
        let calendar_maker = CalendarMaker::from_lines(&mut content.lines());

        let (_, new_calendar, _, _) = CalendarMaker::find_next(
            calendar_maker.availabilities.clone(),
            calendar_maker.calendar.clone(),
            Event::FirstDaily,
            0,
        );
        let names = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ];
        let sorted_names = CalendarMaker::sort_names_by_least_on_call(&names, &new_calendar);
        assert_eq!(sorted_names, vec!["Bob", "Charlie", "Alice"]);
    }
}
