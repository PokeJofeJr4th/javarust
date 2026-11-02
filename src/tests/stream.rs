use super::test;

#[test]
fn test_stream_all() {
    test(
        "TestStreamAll",
        r#"
    java.util.ArrayList<Integer> arr = new java.util.ArrayList<>();
    arr.add(0);
    arr.add(2);
    arr.add(4);
    arr.add(6);
    System.out.println("1");
    assert arr.stream().allMatch((i) -> i % 2 == 0);
    System.out.println("2");
    assert !arr.stream().allMatch((i) -> i > 0);
    System.out.println("3");
    assert !arr.stream().allMatch((i) -> i < 5);
    System.out.println("4");
    assert arr.stream().allMatch((i) -> i < 7);
    "#,
    );
}

#[test]
fn test_stream_any() {
    test(
        "TestStreamAny",
        r#"
    java.util.ArrayList<Integer> arr = new java.util.ArrayList<>();
    arr.add(0);
    arr.add(2);
    arr.add(4);
    arr.add(6);
    System.out.println("1");
    assert arr.stream().anyMatch((i) -> i % 2 == 0);
    System.out.println("2");
    assert arr.stream().anyMatch((i) -> i > 0);
    System.out.println("3");
    assert arr.stream().anyMatch((i) -> i < 5);
    System.out.println("4");
    assert !arr.stream().anyMatch((i) -> i > 7);
    "#,
    );
}

#[test]
fn test_stream_none() {
    test(
        "TestStreamNone",
        r#"
    java.util.ArrayList<Integer> arr = new java.util.ArrayList<>();
    arr.add(0);
    arr.add(2);
    arr.add(4);
    arr.add(6);
    System.out.println("1");
    assert !arr.stream().noneMatch((i) -> i % 2 == 0);
    System.out.println("2");
    assert !arr.stream().noneMatch((i) -> i > 0);
    System.out.println("3");
    assert !arr.stream().noneMatch((i) -> i < 5);
    System.out.println("4");
    assert arr.stream().noneMatch((i) -> i > 7);
    "#,
    );
}

#[test]
fn test_stream_empty() {
    test(
        "TestStreamEmpty",
        r#"
    assert java.util.stream.Stream.empty().noneMatch((i) -> i != null);
    assert java.util.stream.Stream.empty().noneMatch((i) -> i == null);
    assert java.util.stream.Stream.empty().allMatch((i) -> i != null);
    assert java.util.stream.Stream.empty().allMatch((i) -> i == null);
    assert !java.util.stream.Stream.empty().anyMatch((i) -> i != null);
    assert !java.util.stream.Stream.empty().anyMatch((i) -> i == null);
    "#,
    );
}
