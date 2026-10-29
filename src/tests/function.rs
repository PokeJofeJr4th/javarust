use super::test;

#[test]
fn test_function() {
    test(
        "TestFunction",
        r#"
    java.util.function.Function<String, String> x = (s) -> s + " " + s;
    assert x.apply("x") == "x x";
    assert x.apply(x.apply("x")) == "x x x x";
    java.util.function.Function<String, String> y = (s) -> x.apply(s + " ");
    assert y.apply("x") == "x  x ";
    "#,
    );
}

#[test]
fn test_function_compose() {
    test(
        "TestFunctionCompose",
        r#"
    java.util.function.Function<String, String> x = (s) -> s + "v" + s;
    java.util.function.Function<String, String> y = (s) -> "(" + s + ")";
    java.util.function.Function<String, String> z = x.compose(y);
    assert z.apply("^") == "(^)v(^)";
    java.util.function.Function<String, String> w = y.compose(x);
    assert w.apply("^") == "(^v^)";
    "#,
    );
}

#[test]
fn test_function_identity() {
    test(
        "TestFunctionIdentity",
        r#"
    java.util.function.Function<String, String> i = java.util.function.Function.identity();
    assert i.apply("abc") == "abc";
    java.util.function.Function<String, String> x = (s) -> "(" + s + ")";
    java.util.function.Function<String, String> y = x.compose(i);
    assert y.apply("abc") == "(abc)";
    "#,
    );
}

#[test]
fn test_predicate() {
    test(
        "TestFunctionPredicate",
        r#"
    java.util.function.Predicate<String> isLarge = (s) -> s.length() > 3;
    assert isLarge.test("abcdef");
    assert !isLarge.test("abc");
    "#,
    );
}

#[test]
fn test_predicate_not() {
    test(
        "TestFunctionPredicateNot",
        r#"
    java.util.function.Predicate<String> isLarge = (s) -> s.length() > 3;
    java.util.function.Predicate<String> isSmall = isLarge.negate();
    assert isLarge.test("abcdef");
    assert !isSmall.test("abcdef");
    assert !isLarge.test("abc");
    assert isSmall.test("abc");
    "#,
    )
}

#[test]
fn test_predicate_compose() {
    test(
        "TestFunctionPredicateCompose",
        r#"
    java.util.function.Predicate<String> isLarge = (s) -> s.length() > 3;
    java.util.function.Predicate<String> isEven = (s) -> s.length() % 2 == 0;
    java.util.function.Predicate<String> isLargeAndEven = isLarge.and(isEven);
    assert isLargeAndEven.test("abcdef");
    assert !isLargeAndEven.test("abcdefg");
    assert !isLargeAndEven.test("abc");
    assert !isLargeAndEven.test("ab");
    "#,
    );
    test(
        "TestFunctionPredicateCompose",
        r#"
    java.util.function.Predicate<String> isLarge = (s) -> s.length() > 3;
    java.util.function.Predicate<String> isEven = (s) -> s.length() % 2 == 0;
    java.util.function.Predicate<String> isLargeOrEven = isLarge.or(isEven);
    assert isLargeOrEven.test("abcdef");
    assert isLargeOrEven.test("abcdefg");
    assert !isLargeOrEven.test("abc");
    assert isLargeOrEven.test("ab");
    "#,
    );
}
