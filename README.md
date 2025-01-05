# Aubépine

Aubépine is a Rust project designed to create and manage calendars based on availabilities and events. It ensures that all days are filled with the appropriate events and persons, following specific rules.

## Features

- Create calendars from CSV files
- Manage availabilities for multiple persons
- Ensure no person is on-call for consecutive days or events
- Add subcontractors to fill gaps in the calendar

## Installation

To install the project, clone the repository and build it using Cargo:

```sh
git clone git@github.com:robinoffzewood/roseraie-planning.git
cd roseraie-planning
cargo build