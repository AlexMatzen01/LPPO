use lppo::build_executor;

#[test]
fn test_str_concat() {
    let source = r#"
        pub proc main() -> Void {
            let own s1: Str = "Hello";
            let own s2: Str = " World";
            let own result: Str = str_concat(s1, s2);
            out result;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_int_to_str() {
    let source = r#"
        pub proc main() -> Void {
            let own num: Int64 = 42;
            let own result: Str = int_to_str(num);
            out result;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_to_int() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "123";
            let own num: Int64 = str_to_int(s);
            let own doubled: Int64 = num * 2;
            out doubled;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_len() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "Hello";
            let own len: Int64 = str_len(s);
            out len;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_upper_lower() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "Hello";
            let own upper: Str = str_upper(s);
            let own lower: Str = str_lower(s);
            out upper;
            out lower;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_trim() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "  hello  ";
            let own trimmed: Str = str_trim(s);
            out trimmed;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_split() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "a,b,c";
            let own parts: Arr<Str> = str_split(s, ",");
            for part: Str in parts {
                out part;
            }
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());
}

#[test]
fn test_str_to_int_error() {
    let source = r#"
        pub proc main() -> Void {
            let own s: Str = "not_a_number";
            let own num: Int64 = str_to_int(s);
            out num;
        }
    "#;
    
    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    // Should fail because "not_a_number" cannot be parsed as an integer
    assert!(result.is_err());
}
