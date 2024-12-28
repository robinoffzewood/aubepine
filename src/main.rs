use roseraie_planning::CalendarMaker;
fn main() {
    let mut calendar_maker = CalendarMaker::from_file("./tests/files/jan-25.csv");
    calendar_maker.make_calendar(4);
    calendar_maker.print_calendar();
}
