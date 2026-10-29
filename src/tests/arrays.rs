use super::{test, test_fail};

#[test]
fn test_get() {
    test("TestArrayGet", "int[] x={1,2,3};assert x[0]==1;");
    test_fail("TestArrayGet", "int[] x={1,2,3};assert x[4]==1;");
}

#[test]
fn test_get_double() {
    test("TestArrayGetDouble", "double[] x={1.0,3.14159265358,1.618}; assert x[0]==1.0;assert x[1]==3.14159265358;assert x[2]==1.618;");
    test_fail(
        "TestArrayGetDouble",
        "double[] x={1.0,2.0,3.5}; assert x[4] != 0;",
    );
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

#[test]
fn test_set_double() {
    test("TestArraySetDouble", "double[] x={1.0,3.14159265358,1.618}; assert x[1]==3.14159265358;x[1]=6.7;assert x[1]==6.7;");
    test_fail(
        "TestArraySetDouble",
        "double[] x={1.0,3.14,1.618};x[4]=1.0;",
    );
}

#[test]
fn test_array_len() {
    test("TestArrayLen", "int[]x={1,2,3,4}; assert x.length==4;");
    test("TestArrayLen", "float[]x={}; assert x.length==0;");
}

#[test]
fn test_array_len_double() {
    test(
        "TestArrayLenDouble",
        "double[] x={1.0,3.14159,1.618};assert x.length==3;",
    );
}

#[test]
fn test_multi_array() {
    test(
        "TestArrayMulti",
        "
        int[][] x = { {1,2,3}, {4,5,6}, {7,8,9} };
        assert x[1][1] == 5;
        assert x[0][2] == 3;

        int[] y = {10,11,12};
        x[0]=y;
        assert x[0][1] == 11;
        ",
    );
}

#[test]
fn test_array_to_string() {
    test(
        "TestArrayToString",
        "int[] x = {1,2,3};assert java.util.Arrays.toString(x)==\"[1, 2, 3]\";",
    );
}
