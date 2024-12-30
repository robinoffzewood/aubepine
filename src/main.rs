use roseraie_planning::CalendarMaker;
fn main() {
    use std::time::Instant;
    let now = Instant::now();

    let mut calendar_maker = CalendarMaker::from_file("./tests/files/fev-25-7j-dbg.csv");
    // calendar_maker.make_calendar(0);
    calendar_maker.print_results();

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
