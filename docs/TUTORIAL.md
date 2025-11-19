# LPPO v2.0 Tutorial: From Beginner to Advanced

## Introduction

Welcome to LPPO (Low-level Procedural Programming with Ownership)! This tutorial will guide you through learning this deliberately verbose and explicit programming language.

> **Warning**: LPPO is intentionally complex. It is designed to maximize explicitness and minimize convenience. Be prepared for a steep learning curve!

## Table of Contents

1. [Getting Started](#getting-started)
2. [Your First Program](#your-first-program)
3. [Variables and Types](#variables-and-types)
4. [Operations and Expressions](#operations-and-expressions)
5. [Control Flow](#control-flow)
6. [Procedures](#procedures)
7. [Arrays and Collections](#arrays-and-collections)
8. [Memory Management](#memory-management)
9. [Modules and Organization](#modules-and-organization)
10. [Advanced Topics](#advanced-topics)

## Getting Started

### Installation

Make sure you have:
1. Rust toolchain (1.80+ recommended)
2. The LPPO compiler

Build LPPO:
```bash
cargo build --release
```

### Running Programs

Execute an LPPO program:
```bash
cargo run -- program.lppo
```

Compile to standalone executable:
```bash
cargo run -- --compile-out output.exe program.lppo
./output.exe
```

## Your First Program

### Hello World

Every LPPO program starts with a `main` procedure. Let's create `hello.lppo`:

```lppo
pub proc main() -> Void {
    let own msg: Str = "Hello, World!";
    out msg;
}
```

**Key points**:
- `pub proc main() -> Void` - Public procedure named "main" returning nothing
- `let own msg: Str` - Declare owned variable "msg" of type Str
- `= "Hello, World!"` - Assign string literal
- `out msg` - Output the variable

Run it:
```bash
cargo run -- hello.lppo
```

Output:
```
Hello, World!
```

### Multiple Outputs

You can output multiple values at once:

```lppo
pub proc main() -> Void {
    let own greeting: Str = "Hello";
    let own name: Str = "Alice";
    let own exclaim: Str = "!";
    out greeting, name, exclaim;
}
```

Output:
```
Hello Alice !
```

## Variables and Types

### Declaring Variables

**EVERY variable MUST have an explicit type annotation**. There is NO type inference.

```lppo
// WRONG - No type:
// let own x = 10;  // ERROR!

// CORRECT - Explicit type:
let own x: Int64 = 10;
```

### Basic Types

**Int64**: 64-bit integers
```lppo
let own age: Int64 = 25;
let own negative: Int64 = -100;
let own zero: Int64 = 0;
```

**Bool**: Boolean values
```lppo
let own is_true: Bool = true;
let own is_false: Bool = false;
```

**Str**: Strings
```lppo
let own message: Str = "Hello";
let own empty: Str = "";
```

**Void**: No value (for procedure returns)
```lppo
pub proc do_nothing() -> Void {
    // No return needed
}
```

### Ownership Qualifiers

Every variable has an ownership qualifier:

**own**: Owned value (most common)
```lppo
let own x: Int64 = 42;
```

**mut**: Mutable (can be changed)
```lppo
let mut own counter: Int64 = 0;
```

### Exercise 1: Variables

Create a program that declares multiple variables of different types and outputs them:

```lppo
pub proc main() -> Void {
    let own name: Str = "Bob";
    let own age: Int64 = 30;
    let own is_student: Bool = false;
    
    let own label1: Str = "Name:";
    let own label2: Str = "Age:";
    let own label3: Str = "Student:";
    
    out label1, name;
    out label2, age;
    out label3, is_student;
}
```

## User Input

LPPO provides the `input` statement for reading user input. Like everything in LPPO, **input requires explicit type annotation**.

### Basic Input

The simplest form reads input without a prompt:

```lppo
pub proc main() -> Void {
    input name: Str;
    out "Hello,", name;
}
```

### Input with Prompt

Use `with` to display a prompt before reading:

```lppo
pub proc main() -> Void {
    input name: Str with "What is your name?";
    let own greeting: Str = "Welcome,";
    out greeting, name;
}
```

### Input Types

**String Input:**
```lppo
pub proc main() -> Void {
    input message: Str with "Enter a message:";
    out "You said:", message;
}
```

**Integer Input:**
```lppo
pub proc main() -> Void {
    input number: Int64 with "Enter a number:";
    let own doubled: Int64 = number + number;
    out "Double:", doubled;
}
```

**Boolean Input:**
```lppo
pub proc main() -> Void {
    input continue_flag: Bool with "Continue? (yes/no):";
    let own yes_msg: Str = "Continuing...";
    let own no_msg: Str = "Stopping...";
    
    if continue_flag {
        out yes_msg;
    } else {
        out no_msg;
    }
}
```

Boolean input accepts: `true`, `false`, `yes`, `no`, `y`, `n`, `1`, `0` (case-insensitive)

**Array Input:**
```lppo
pub proc main() -> Void {
    input numbers: Arr<Int64> with "Enter numbers (comma-separated):";
    out "You entered:", numbers;
    
    // Process the array
    for num: Int64 in numbers {
        out "Processing:", num;
    }
}
```

Array input accepts:
- Comma-separated: `1,2,3,4,5`
- Bracket notation: `[1, 2, 3, 4, 5]`

### Exercise 1.5: Interactive Calculator

Create a calculator that reads two numbers and displays their sum:

```lppo
pub proc main() -> Void {
    input num1: Int64 with "Enter first number:";
    input num2: Int64 with "Enter second number:";
    
    let own sum: Int64 = num1 + num2;
    let own product: Int64 = num1 * num2;
    
    out "Sum:", sum;
    out "Product:", product;
}
```

## Pause / Timers

LPPO provides a small `pause` statement for simple timing and wait-for-input use cases. It can be used as a one-off sleep, an input-with-timeout, or (with `async`) to schedule remaining work later.

Synchronous sleep:

```lppo
pub proc main() -> Void {
    out "Sleeping...";
    // block current procedure for 500 ms
    pause { timeout 500; }
    out "Woke up";
}
```

Input with timeout (assign result to variable):

```lppo
pub proc main() -> Void {
    out "Type 'go' within 3s:";
    pause { assign_to answer: Str; timeout 3000; }
    out "You answered:", answer;
}
```

Asynchronous pause (schedule remaining operations):

```lppo
pub proc main() -> Void {
    out "Timer test";
    // Remaining operations will run after 1000ms in background
    pause { async; timeout 1000; }
    out "Main returns immediately (remaining ops scheduled)";
}
```

Notes:
- The `async` form schedules the *remaining operations* of the current procedure to run in a background worker after the timeout. The background worker receives a cloned (snapshot) environment and a cloned procedure map.
- By default the interpreter waits for background continuations to finish before exiting, so output from scheduled work is preserved. If you want true fire-and-forget semantics you can change the runtime to not join background handles or add an explicit API to wait for them.

### Error Handling

Input validation is automatic:
- Invalid integers produce: `"Invalid Int64 input: 'abc'"`
- Invalid booleans produce: `"Invalid Bool input: 'maybe'. Expected: true/false, yes/no, 1/0"`
- Invalid array elements produce: `"Invalid Int64 in array: 'xyz'"`

## Operations and Expressions

### Arithmetic

**All operands must be explicitly typed**:

```lppo
pub proc main() -> Void {
    let own a: Int64 = 10;
    let own b: Int64 = 3;
    
    let own sum: Int64 = a + b;          // 13
    let own diff: Int64 = a - b;         // 7
    let own prod: Int64 = a * b;         // 30
    let own quot: Int64 = a / b;         // 3
    let own rem: Int64 = a % b;          // 1
    
    let own two: Int64 = 2;
    let own pow: Int64 = a ** two;       // 100
    
    let own label_sum: Str = "Sum:";
    let own label_prod: Str = "Product:";
    out label_sum, sum;
    out label_prod, prod;
}
```

### Comparison

**Results must be stored in Bool variables**:

```lppo
pub proc main() -> Void {
    let own x: Int64 = 10;
    let own y: Int64 = 20;
    
    let own eq: Bool = x == y;        // false
    let own ne: Bool = x != y;        // true
    let own lt: Bool = x < y;         // true
    let own gt: Bool = x > y;         // false
    
    out "x equals y:", eq;
    out "x less than y:", lt;
}
```

### Logical Operations

```lppo
pub proc main() -> Void {
    let own a: Bool = true;
    let own b: Bool = false;
    
    let own and_result: Bool = a and b;    // false
    let own or_result: Bool = a or b;      // true
    let own not_result: Bool = not a;      // false
    
    out "AND:", and_result;
    out "OR:", or_result;
    out "NOT:", not_result;
}
```

### Exercise 2: Calculator

Create a simple calculator:

```lppo
pub proc main() -> Void {
    let own num1: Int64 = 15;
    let own num2: Int64 = 4;
    
    let own sum: Int64 = num1 + num2;
    let own diff: Int64 = num1 - num2;
    let own prod: Int64 = num1 * num2;
    let own quot: Int64 = num1 / num2;
    let own rem: Int64 = num1 % num2;
    
    out "Addition:", sum;
    out "Subtraction:", diff;
    out "Multiplication:", prod;
    out "Division:", quot;
    out "Remainder:", rem;
}
```

## Control Flow

### If Statements

**Conditions must be explicit Bool variables**:

```lppo
pub proc main() -> Void {
    let own x: Int64 = 15;
    let own threshold: Int64 = 10;
    
    // Store comparison result
    let own is_large: Bool = x > threshold;
    
    if is_large {
        let own msg: Str = "x is large";
        out msg;
    } else {
        let own msg: Str = "x is small";
        out msg;
    }
}
```

### Multiple Conditions (elif)

```lppo
pub proc main() -> Void {
    let own score: Int64 = 75;
    let own ninety: Int64 = 90;
    let own eighty: Int64 = 80;
    let own seventy: Int64 = 70;
    
    let own is_a: Bool = score >= ninety;
    let own is_b: Bool = score >= eighty;
    let own is_c: Bool = score >= seventy;
    
    if is_a {
        out "Grade: A";
    } elif is_b {
        out "Grade: B";
    } elif is_c {
        out "Grade: C";
    } else {
        out "Grade: F";
    }
}
```

### While Loops

```lppo
pub proc main() -> Void {
    let mut own i: Int64 = 0;
    let own limit: Int64 = 5;
    
    while i < limit {
        let own label: Str = "Count:";
        out label, i;
        
        // Increment using move
        let own one: Int64 = 1;
        let own i_next: Int64 = i + one;
        move i_next -> i;
    }
}
```

### For Loops

**Loop variable must have explicit type**:

```lppo
pub proc main() -> Void {
    let own numbers: Arr<Int64> = [10, 20, 30, 40, 50];
    
    for num: Int64 in numbers {
        let own label: Str = "Number:";
        out label, num;
    }
}
```

### Infinite Loops with Break

```lppo
pub proc main() -> Void {
    let mut own count: Int64 = 0;
    let own max: Int64 = 3;
    let own one: Int64 = 1;
    
    loop {
        out "Iteration:", count;
        
        let own count_next: Int64 = count + one;
        move count_next -> count;
        
        let own should_break: Bool = count >= max;
        if should_break {
            break;
        }
    }
    
    out "Loop finished";
}
```

### Exercise 3: Countdown

Create a countdown from 10 to 1:

```lppo
pub proc main() -> Void {
    let mut own i: Int64 = 10;
    let own zero: Int64 = 0;
    let own one: Int64 = 1;
    
    while i > zero {
        out i;
        let own i_next: Int64 = i - one;
        move i_next -> i;
    }
    
    out "Blast off!";
}
```

## Procedures

### Defining Procedures

**All procedures require explicit types for everything**:

```lppo
pub proc greet(name: Str, age: Int64) -> Void {
    let own hello: Str = "Hello";
    let own you_are: Str = "You are";
    let own years_old: Str = "years old";
    
    out hello, name, "!";
    out you_are, age, years_old;
}

pub proc main() -> Void {
    let own person_name: Str = "Alice";
    let own person_age: Int64 = 25;
    
    call greet(person_name, person_age);
}
```

### Procedures Without Parameters

```lppo
pub proc say_hello() -> Void {
    let own msg: Str = "Hello from procedure!";
    out msg;
}

pub proc main() -> Void {
    call say_hello();
}
```

### Exercise 4: Temperature Converter

```lppo
pub proc celsius_to_fahrenheit(celsius: Int64) -> Void {
    let own nine: Int64 = 9;
    let own five: Int64 = 5;
    let own thirty_two: Int64 = 32;
    
    let own temp1: Int64 = celsius * nine;
    let own temp2: Int64 = temp1 / five;
    let own fahrenheit: Int64 = temp2 + thirty_two;
    
    let own label1: Str = "Celsius:";
    let own label2: Str = "Fahrenheit:";
    
    out label1, celsius;
    out label2, fahrenheit;
}

pub proc main() -> Void {
    let own temp: Int64 = 25;
    call celsius_to_fahrenheit(temp);
}
```

## Arrays and Collections

### Creating Arrays

**Element type must be explicit**:

```lppo
pub proc main() -> Void {
    let own numbers: Arr<Int64> = [1, 2, 3, 4, 5];
    let own words: Arr<Str> = ["hello", "world"];
    
    out numbers;
    out words;
}
```

### Array Indexing

```lppo
pub proc main() -> Void {
    let own arr: Arr<Int64> = [10, 20, 30, 40, 50];
    
    // Positive indexing
    let own zero: Int64 = 0;
    let own two: Int64 = 2;
    let own first: Int64 = arr[zero];
    let own third: Int64 = arr[two];
    
    // Negative indexing (from end)
    let own neg_one: Int64 = -1;
    let own last: Int64 = arr[neg_one];
    
    out "First:", first;
    out "Third:", third;
    out "Last:", last;
}
```

### Array Methods

```lppo
pub proc main() -> Void {
    let own arr: Arr<Int64> = [1, 2, 3, 4, 5];
    
    // Length
    let own len: Int64 = arr.len();
    out "Length:", len;
    
    // Concatenation
    let own arr2: Arr<Int64> = [6, 7, 8];
    let own combined: Arr<Int64> = arr + arr2;
    out "Combined:", combined;
}
```

### String Operations

```lppo
pub proc main() -> Void {
    let own msg: Str = "Hello World";
    
    // Length
    let own len: Int64 = msg.len();
    out "Length:", len;
    
    // Case conversion
    let own upper: Str = msg.upper();
    let own lower: Str = msg.lower();
    out "Upper:", upper;
    out "Lower:", lower;
    
    // Split
    let own text: Str = "a,b,c,d";
    let own sep: Str = ",";
    let own parts: Arr<Str> = text.split(sep);
    out "Parts:", parts;
}
```

### Exercise 5: Array Processing

```lppo
pub proc process_array(arr: Arr<Int64>) -> Void {
    let own label: Str = "Processing array:";
    out label, arr;
    
    let own len: Int64 = arr.len();
    out "Length:", len;
    
    let own zero: Int64 = 0;
    let own neg_one: Int64 = -1;
    let own first: Int64 = arr[zero];
    let own last: Int64 = arr[neg_one];
    
    out "First element:", first;
    out "Last element:", last;
}

pub proc main() -> Void {
    let own data: Arr<Int64> = [5, 10, 15, 20, 25];
    call process_array(data);
}
```

## Memory Management

### Mutable Variables

```lppo
pub proc main() -> Void {
    let mut own counter: Int64 = 0;
    let own one: Int64 = 1;
    let own limit: Int64 = 3;
    
    while counter < limit {
        out "Counter:", counter;
        
        let own next: Int64 = counter + one;
        move next -> counter;
    }
}
```

### Move Semantics

Transfer ownership:

```lppo
pub proc main() -> Void {
    let own source: Int64 = 42;
    let own target: Int64 = 0;
    
    out "Before move:", source, target;
    
    move source -> target;
    
    // source is now invalid
    out "After move:", target;
}
```

### Copy Semantics

Duplicate values:

```lppo
pub proc main() -> Void {
    let own original: Int64 = 100;
    let own duplicate: Int64 = 0;
    
    copy original -> duplicate;
    
    // Both are valid
    out "Original:", original;
    out "Duplicate:", duplicate;
}
```

## Modules and Organization

### Defining Modules

```lppo
module math {
    pub proc add(a: Int64, b: Int64) -> Void {
        let own result: Int64 = a + b;
        let own label: Str = "Sum:";
        out label, result;
    }
    
    pub proc multiply(a: Int64, b: Int64) -> Void {
        let own result: Int64 = a * b;
        let own label: Str = "Product:";
        out label, result;
    }
}

pub proc main() -> Void {
    let own x: Int64 = 5;
    let own y: Int64 = 3;
    
    call math::add(x, y);
    call math::multiply(x, y);
}
```

### Nested Modules

```lppo
module utilities {
    module string_utils {
        pub proc concat_with_space(a: Str, b: Str) -> Void {
            let own space: Str = " ";
            let own temp: Str = a + space;
            let own result: Str = temp + b;
            out result;
        }
    }
}

pub proc main() -> Void {
    let own word1: Str = "Hello";
    let own word2: Str = "World";
    
    call utilities::string_utils::concat_with_space(word1, word2);
}
```

## Advanced Topics

### Parallel Execution

```lppo
pub proc main() -> Void {
    scope parallel_section {
        parallel {
            out "Task 1";
            out "Task 2";
            out "Task 3";
        }
    }
    
    out "All tasks completed";
}
```

### Explicit Scopes

```lppo
pub proc main() -> Void {
    scope outer {
        let own x: Int64 = 10;
        out "Outer x:", x;
        
        scope inner {
            let own y: Int64 = 20;
            out "Inner y:", y;
            
            let own sum: Int64 = x + y;
            out "Sum:", sum;
        }
        
        // y is no longer accessible here
        out "Back in outer scope";
    }
}
```

### Unsafe Operations

```lppo
pub proc main() -> Void {
    unsafe {
        // Low-level operations that bypass safety checks
        let own msg: Str = "In unsafe block";
        out msg;
    }
}
```

## Complete Example: Number Analyzer

```lppo
pub proc analyze_number(n: Int64) -> Void {
    let own label: Str = "Analyzing:";
    out label, n;
    
    // Check if positive
    let own zero: Int64 = 0;
    let own is_positive: Bool = n > zero;
    
    if is_positive {
        out "Number is positive";
    } else {
        out "Number is not positive";
    }
    
    // Check if even
    let own two: Int64 = 2;
    let own remainder: Int64 = n % two;
    let own is_even: Bool = remainder == zero;
    
    if is_even {
        out "Number is even";
    } else {
        out "Number is odd";
    }
}

pub proc main() -> Void {
    let own numbers: Arr<Int64> = [5, -3, 10, 0, -8];
    
    for num: Int64 in numbers {
        call analyze_number(num);
        out "---";
    }
}
```

## Next Steps

1. **Practice**: Write small programs for each concept
2. **Experiment**: Try combining different features
3. **Read**: Study the Language Specification for details
4. **Build**: Create larger programs using modules

## Common Mistakes

### 1. Forgetting Type Annotations

```lppo
// WRONG:
// let own x = 10;

// CORRECT:
let own x: Int64 = 10;
```

### 2. Using Expressions in Conditions

```lppo
// WRONG:
// if x > 10 { }

// CORRECT:
let own ten: Int64 = 10;
let own condition: Bool = x > ten;
if condition { }
```

### 3. Forgetting Loop Variable Types

```lppo
// WRONG:
// for item in array { }

// CORRECT:
for item: Int64 in array { }
```

### 4. Missing Ownership Qualifier

```lppo
// WRONG:
// let x: Int64 = 10;

// CORRECT:
let own x: Int64 = 10;
```

## Tips for Success

1. **Be explicit**: Never assume the compiler will figure things out
2. **Name intermediate variables**: Break complex expressions into steps
3. **Use descriptive names**: Make your code self-documenting
4. **Manage scope**: Use scope blocks to organize code
5. **Plan ahead**: Think about ownership and lifetimes early

Happy LPPO programming!
