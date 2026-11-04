use super::test;

#[test]
fn test_constants() {
    test(
        "TestMathConstants",
        r#"
        double e = java.lang.Math.E;
        double pi = java.lang.Math.PI;
        double tau = java.lang.Math.TAU;
        assert pi > 3;
        assert e < pi;
        assert pi > 3.14159265358;
        assert pi < 3.14159265359;
        assert e > 2.718;
        assert e < 2.719;
        assert tau > pi;
        assert tau == 2 * pi;
        assert tau / 2 == pi;
    "#,
    );
}

#[test]
fn test_sqrt_double() {
    test(
        "TestMathSqrt",
        r#"
    double x = 100;
    assert java.lang.Math.sqrt(x) == 10;
    "#,
    );
}
