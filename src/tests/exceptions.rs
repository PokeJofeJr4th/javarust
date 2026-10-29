use super::{test, test_fail};

#[test]
fn test_throw() {
    test_fail(
        "TestExceptionThrow",
        r#"throw new IllegalArgumentException();"#,
    );
}

#[test]
fn test_catch() {
    test(
        "TestExceptionCatch",
        r#"
    try {
        if (Integer.valueOf(21).intValue()==21) {
            throw new IllegalArgumentException();
        }
    } catch (IllegalArgumentException e) {
        System.out.println("Caught exception");
        return;
    }
    assert false;
    "#,
    );
    test_fail(
        "TestExceptionCatch",
        r#"
    try {
        throw new IllegalArgumentException();
    } catch (ArithmeticException e) {
        System.out.println("Caught exception");
        return;
    }
    "#,
    )
}
