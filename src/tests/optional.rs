use super::test;

#[test]
fn test_optional() {
    test(
        "TestOptional",
        r#"
    java.util.Optional<String> x = java.util.Optional.of("abcdef");
    assert !x.isEmpty();
    assert x.isPresent();
    java.util.Optional<String> y = java.util.Optional.empty();
    assert y.isEmpty();
    assert !y.isPresent();
    "#,
    );
}

#[test]
fn test_optional_of_nullable() {
    test(
        "TestOptionalOfNullable",
        r#"
    java.util.Optional<String> x = java.util.Optional.ofNullable("abcdef");
    assert !x.isEmpty();
    assert x.isPresent();
    java.util.Optional<String> y = java.util.Optional.ofNullable(null);
    assert y.isEmpty();
    assert !y.isPresent();
    "#,
    );
}

#[test]
fn test_optional_filter() {
    test(
        "TestOptionalFilter",
        r#"
    java.util.function.Predicate<String> isLong = (s) -> s.length() > 3;
    java.util.Optional<String> x = java.util.Optional.of("abcdef");
    java.util.Optional<String> y = java.util.Optional.of("abc");
    java.util.Optional<String> xp = x.filter(isLong);
    java.util.Optional<String> yp = y.filter(isLong);
    assert xp.isPresent();
    assert !xp.isEmpty();
    assert !yp.isPresent();
    assert yp.isEmpty();
    "#,
    );
}

#[test]
fn test_optional_map() {
    test(
        "TestOptionalMap",
        r#"
    java.util.function.Function<String, String> mapper = (s) -> "(" + s + ")";
    java.util.Optional<String> x = java.util.Optional.of("abcdef");
    System.out.println("--- OPTVALUE=" + x);
    java.util.Optional<String> xm = x.map(mapper);
    System.out.println("--- OPTVALUE=" + xm);
    assert xm.isPresent();
    assert !xm.isEmpty();
    assert xm.toString() == "Optional[(abcdef)]";
    "#,
    );
}
