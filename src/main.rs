use clap::Parser;
use roseraie_planning::CalendarMaker;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File path to the CSV file
    #[arg(short, long)]
    filename: String,

    /// Max number of subcontractors
    #[arg(short, long, default_value_t = 0)]
    subco: u8,

    // Verbosity
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    use std::time::Instant;
    let now = Instant::now();

    let mut calendar_maker = CalendarMaker::from_file(&args.filename);
    calendar_maker.make_calendar(args.subco, args.verbose);
    calendar_maker.print_results();

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
