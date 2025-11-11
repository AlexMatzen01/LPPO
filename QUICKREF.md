# LPPO Quick Reference

## Syntax at a Glance

### Variables
```lppo
let x = 42;
let name = "Alice";
let flag = true;
let nums = [1, 2, 3];
```

### Output
```lppo
out "Hello";
out "Values:", x, y, z;
out arr;
```

### Procedures
```lppo
proc name {
    // body
}

proc greet(name, age) {
    out "Hello", name, "Age:", age;
}

call greet("Bob", 25);
```

### Conditionals
```lppo
if condition {
    // then
} elif other_condition {
    // else if
} else {
    // else
}
```

### Loops
```lppo
// While loop
while i < 10 {
    out i;
    let i = i + 1;
}

// For-in loop
for item in array {
    out item;
}

// Infinite loop
loop {
    // ...
    break;
}

// Control flow
break;      // Exit loop
continue;   // Skip to next iteration
```

### Operators

**Arithmetic**
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division
- `%` Modulo
- `**` Power

**Comparison**
- `==` Equal
- `!=` Not equal
- `<` Less than
- `>` Greater than
- `<=` Less or equal
- `>=` Greater or equal

**Logical**
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

**Bitwise**
- `&` Bitwise AND
- `|` Bitwise OR
- `^` Bitwise XOR
- `~` Bitwise NOT

**Unary**
- `-` Negation
- `!` Logical NOT
- `~` Bitwise NOT

### Arrays
```lppo
let arr = [1, 2, 3, 4, 5];

// Indexing
arr[0]      // First: 1
arr[-1]     // Last: 5

// Methods
arr.len()   // Length: 5

// Concatenation
[1, 2] + [3, 4]  // [1, 2, 3, 4]
```

### Strings
```lppo
let s = "Hello World";

// Indexing
s[0]        // "H"
s[-1]       // "d"

// Methods
s.len()     // 11
s.upper()   // "HELLO WORLD"
s.lower()   // "hello world"
s.split(" ") // ["Hello", "World"]

// Concatenation
"hello" + " " + "world"  // "hello world"
```

### Parallel Execution
```lppo
parallel {
    out "Task 1";
    out "Task 2";
    out "Task 3";
}
```

### Comments
```lppo
// Single line comment

/* 
   Multi-line
   comment
*/
```

## Operator Precedence

(Highest to lowest)

1. Unary: `-`, `!`, `~`
2. Power: `**`
3. Multiplicative: `*`, `/`, `%`
4. Additive: `+`, `-`
5. Bitwise: `&`, `|`, `^`
6. Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
7. Logical AND: `and`
8. Logical OR: `or`

Use parentheses `()` to override precedence.

## Data Types

- **Integer**: `42`, `-17`, `0`
- **Boolean**: `true`, `false`
- **String**: `"text"`, `"hello\nworld"`
- **Array**: `[1, 2, 3]`, `["a", "b"]`

### Escape Sequences in Strings

- `\n` - Newline
- `\r` - Carriage return
- `\t` - Tab
- `\\` - Backslash
- `\"` - Double quote

## Built-in Methods

### Array Methods
- `.len()` - Returns length

### String Methods
- `.len()` - Returns length
- `.upper()` - Convert to uppercase
- `.lower()` - Convert to lowercase
- `.split(separator)` - Split into array

## Command Line Usage

```bash
# Run program
cargo run -- program.lppo

# Compile to executable
cargo run -- --compile-out output.exe program.lppo

# Debug mode
cargo run -- --debug program.lppo
```

## Common Patterns

### Loop over array with index
```lppo
let items = ["a", "b", "c"];
let i = 0;
while i < items.len() {
    out "Index", i, "Value", items[i];
    let i = i + 1;
}
```

### Find element in array
```lppo
let nums = [1, 2, 3, 4, 5];
let target = 3;
let found = false;

for n in nums {
    if n == target {
        out "Found:", n;
        let found = true;
        break;
    }
}

if not found {
    out "Not found";
}
```

### Process even numbers only
```lppo
for n in [1, 2, 3, 4, 5, 6] {
    if n % 2 != 0 {
        continue;
    }
    out "Even:", n;
}
```

### Nested loops
```lppo
let i = 0;
while i < 3 {
    let j = 0;
    while j < 3 {
        out "i =", i, "j =", j;
        let j = j + 1;
    }
    let i = i + 1;
}
```
