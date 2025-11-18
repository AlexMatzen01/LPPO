/// Integration tests for strict type and ownership checking
use lppo::{parse_program, typecheck_program, LppoError};

#[test]
fn test_missing_type_annotation_rejected() {
    let source = "pub proc main() -> Void { let own x = 42; }";
    let result = parse_program(source);
    assert!(result.is_err());
    match result {
        Err(LppoError::SyntaxError { message, .. }) => {
            assert!(message.contains("missing type annotation"));
        }
        _ => panic!("Expected SyntaxError"),
    }
}

#[test]
fn test_type_mismatch_rejected() {
    let source = r#"
        pub proc main() -> Void {
            let own x: Int64 = 42;
            let own y: Bool = x;
        }
    "#;
    let ast = parse_program(source).expect("should parse");
    let result = typecheck_program(&ast);
    assert!(result.is_err());
    match result {
        Err(LppoError::TypeError { .. }) => {}
        _ => panic!("Expected TypeError"),
    }
}

#[test]
fn test_valid_program_compiles() {
    let source = r#"
        pub proc add(a: Int64, b: Int64) -> Int64 {
            let own result: Int64 = a + b;
            return result;
        }
        
        pub proc main() -> Void {
            let own x: Int64 = 10;
            let own y: Int64 = 20;
            call add(x, y);
            
            let own numbers: Arr<Int64> = [1, 2, 3, 4, 5];
            for num: Int64 in numbers {
                let own doubled: Int64 = num * 2;
                out doubled;
            }
            
            let own is_positive: Bool = x > 0;
            if is_positive {
                out "Positive!";
            }
        }
    "#;
    let ast = parse_program(source).expect("should parse");
    let result = typecheck_program(&ast);
    assert!(result.is_ok(), "Valid program should pass type checking");
}
