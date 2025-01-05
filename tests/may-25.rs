use roseraie_planning::CalendarMaker;

#[test]
fn test_main_for_may_2025() {
    let mut calendar_maker = CalendarMaker::from_file("./tests/files/mai-25-15j.csv");
    let max_subco = 2;
    let verbose = false;
    calendar_maker.make_calendar(max_subco, verbose);
    let mut expected_calendar = "     |  05  |  06  |  07  |  08  |  09  |  10  |  11  |  12  |  13  |  14  |  15  |  16  |  17  |  18  |  19  |  20  |\r\n".to_string();
    expected_calendar.push_str("----------------------------------------------------------------------------------------------------------------------\r\n");
    expected_calendar.push_str("J    | AST  | CIN  | AMA  | CAR  | MEL  | LUX  | ELF  | ALI  | JUL  | AFI  | JEK  | SOS  | ALI  | CAR  | AFI  | AST  |\r\n");
    expected_calendar.push_str("N    | JUL  | ELF  | JEK  | CIN  | AMA  | SOS  | AST  | LUX  | CAR  | CIN  | AMA  | JUL  | AFI  | MEL  | ELF  | ALI  |\r\n");
    expected_calendar.push_str("j    | JEK  | MEL  | LUX  | SOS  | AST  | CIN  | AMA  | JEK  | AST  | ELF  | CAR  | MEL  | CIN  | JUL  | LUX  | CAR  |\r\n");
    expected_calendar.push_str("n    | LUX  | CAR  | JUL  | ELF  | JEK  | CIN  | CAR  | CIN  | SOS  | LUX  | AST  | MEL  | ELF  | JUL  | JEK  | SOS  |\r\n");
    assert_eq!(
        expected_calendar.to_string(),
        calendar_maker.calendar_as_string()
    );
}
