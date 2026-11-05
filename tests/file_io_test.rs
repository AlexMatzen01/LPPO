use lppo::build_executor;
use std::fs;
use std::path::Path;

#[test]
fn test_file_write_and_read() {
    let test_file = "/tmp/lppo_test_write_read.txt";
    // Clean up if exists
    let _ = fs::remove_file(test_file);

    let source = format!(
        r#"
        pub proc main() -> Void {{
            let own path: Str = "{}";
            let own content: Str = "Test content";
            let own write_result: Bool = write_file(path, content);
            let own read_content: Str = read_file(path);
            out read_content;
        }}
    "#,
        test_file
    );

    let mut executor = build_executor(&source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());

    // Verify file was created
    assert!(Path::new(test_file).exists());

    // Verify content
    let content = fs::read_to_string(test_file).unwrap();
    assert_eq!(content, "Test content");

    // Clean up
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_file_append() {
    let test_file = "/tmp/lppo_test_append.txt";
    // Clean up if exists
    let _ = fs::remove_file(test_file);

    let source = format!(
        r#"
        pub proc main() -> Void {{
            let own path: Str = "{}";
            let own content1: Str = "Line 1";
            let own write_result: Bool = write_file(path, content1);
            let own content2: Str = "\nLine 2";
            let own append_result: Bool = append_file(path, content2);
            let own final_content: Str = read_file(path);
            out final_content;
        }}
    "#,
        test_file
    );

    let mut executor = build_executor(&source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());

    // Verify content
    let content = fs::read_to_string(test_file).unwrap();
    assert_eq!(content, "Line 1\nLine 2");

    // Clean up
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_file_exists() {
    let test_file = "/tmp/lppo_test_exists.txt";
    // Clean up if exists
    let _ = fs::remove_file(test_file);

    let source = format!(
        r#"
        pub proc main() -> Void {{
            let own path: Str = "{}";
            let own exists_before: Bool = file_exists(path);
            let own content: Str = "Test";
            let own write_result: Bool = write_file(path, content);
            let own exists_after: Bool = file_exists(path);
            out exists_after;
        }}
    "#,
        test_file
    );

    let mut executor = build_executor(&source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());

    // Verify file exists
    assert!(Path::new(test_file).exists());

    // Clean up
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_file_delete() {
    let test_file = "/tmp/lppo_test_delete.txt";
    // Create a file first
    fs::write(test_file, "test").unwrap();

    let source = format!(
        r#"
        pub proc main() -> Void {{
            let own path: Str = "{}";
            let own delete_result: Bool = delete_file(path);
            let own exists_after: Bool = file_exists(path);
            out exists_after;
        }}
    "#,
        test_file
    );

    let mut executor = build_executor(&source).unwrap();
    let result = executor.run("main");
    assert!(result.is_ok());

    // Verify file was deleted
    assert!(!Path::new(test_file).exists());
}

#[test]
fn test_read_nonexistent_file() {
    let source = r#"
        pub proc main() -> Void {
            let own path: Str = "/tmp/nonexistent_file_lppo_test.txt";
            let own content: Str = read_file(path);
            out content;
        }
    "#;

    let mut executor = build_executor(source).unwrap();
    let result = executor.run("main");
    assert!(result.is_err());
}
