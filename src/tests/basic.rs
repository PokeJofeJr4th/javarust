use super::{test, test_fail};

#[test]
fn test_one_plus_one() {
    test("TestBasic1", "assert Integer.valueOf(1).intValue()+1==2;");
    test_fail("TestBasic1", "assert Integer.valueOf(1).intValue()+1==3;");
}

#[test]
fn test_for_loop() {
    test(
        "TestBasic2",
        "
        int x=Integer.valueOf(0).intValue();
        for(int i=0;i<10;i++) {
            x+=i;
        }
        System.out.println(x);
        assert x==45;
        ",
    );
}

#[test]
fn test_string_concatenation() {
    test(
        "TestBasic3",
        r#"
        String x="abc123";
        String y="abc"+Integer.valueOf(123).intValue();
        assert x==y;
        "#,
    );
}

#[test]
fn test_reflect() {
    test(
        "TestReflectClassName",
        r#"
        java.lang.Class cls = new TestReflectClassName().getClass();
        System.out.println("cls="+cls);
        System.out.println("cls.getName()="+cls.getName());
        assert cls.getName() == "TestReflectClassName";
        "#,
    );
}
