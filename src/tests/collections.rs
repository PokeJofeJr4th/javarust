use super::test;

#[test]
fn test_hash_map() {
    test(
        "TestHashMap",
        "
        java.util.HashMap<String, Integer> x = new java.util.HashMap<>();
        assert x.size() == 0;
        assert x.get(\"foo\") == null;
        x.put(\"foo\", 10);
        assert x.get(\"foo\") != null;
        assert x.size() == 1;
        ",
    );
}

#[test]
fn test_hash_set() {
    test(
        "TestHashSet",
        "
        java.util.HashSet<String> x = new java.util.HashSet<>();
        assert x.size() == 0;
        assert !x.contains(\"shark\");
        x.add(\"shark\");
        assert x.size() == 1;
        assert x.contains(\"shark\");
        ",
    );
}

#[test]
fn test_array_list() {
    test(
        "TestArrayList",
        r#"
        java.util.ArrayList<String> x = new java.util.ArrayList<>();
        assert x.size() == 0;
        x.add("A");
        assert x.size() == 1;
        x.add("B");
        x.add("D");
        x.add("C");
        assert x.size() == 4;
        "#,
    )
}

#[test]
fn test_array_list_sort() {
    test(
        "TestArrayListSort",
        r#"
        java.util.ArrayList<Integer> x = new java.util.ArrayList<>();
        x.add(1);
        x.add(5);
        x.add(3);
        x.add(7);
        x.add(2);
        x.add(10);
        x.add(8);
        x.add(9);
        x.add(6);
        x.add(4);
        assert x.size() == 10;
        x.sort((a,b) -> a - b);
        assert x.size() == 10;
        System.out.println(x.toString());
        assert x.toString() == "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]";
        "#
    )
}
