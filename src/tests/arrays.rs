use super::{test, test_fail};

#[test]
fn test_get() {
    test("TestArrayGet", "int[] x={1,2,3};assert x[0]==1;");
    test_fail("TestArrayGet", "int[] x={1,2,3};assert x[4]==1;");
}

#[test]
fn test_set() {
    test(
        "TestArraySet",
        "int[] x={1,2,3};assert x[0]==1;x[0]=-1;assert x[0]==-1;",
    );
    test_fail("TestArraySet", "int[] x={1,2,3}; x[4]=-1;");
    test(
        "TestArraySet",
        "int[] x={1,2,3}; assert x[2] == 3; x[2] += 10; assert x[2] == 13;",
    );
}
