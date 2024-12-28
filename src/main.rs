use roseraie_planning::CalendarMaker;
fn main() {
    use std::time::Instant;
    let now = Instant::now();

    let mut calendar_maker = CalendarMaker::from_file("./tests/files/jan-25.csv");
    calendar_maker.make_calendar(2);
    calendar_maker.print_calendar();

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
