use rstest::*;

#[fixture]
fn f1() -> u32 { 0 }
#[fixture]
fn f2() -> u32 { 0 }
#[fixture]
fn f3() -> u32 { 0 }

#[fixture]
fn fixture(f1: u32, f2: u32, f3: u32) -> u32 { f1 + 10 * f2 + 100 * f3 }

#[rstest_parametrize(expected, case(0),  case(1000))]
fn default(fixture: u32, expected: u32) {
    assert_eq!(fixture, expected);
}

#[rstest_parametrize(fixture(7), expected, case(7),  case(1000))]
fn partial_1(fixture: u32, expected: u32) {
    assert_eq!(fixture, expected);
}

#[rstest_parametrize(fixture(2, 4), expected, case(42),  case(1000))]
fn partial_2(fixture: u32, expected: u32) {
    assert_eq!(fixture, expected);
}

#[rstest_parametrize(fixture(2, 4, 5), expected, case(542),  case(1000))]
fn complete(fixture: u32, expected: u32) {
    assert_eq!(fixture, expected);
}
