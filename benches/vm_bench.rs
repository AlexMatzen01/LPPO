use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lppo::{build_executor, compile_to_bytes_with_options, CompileOptions, load_executor_from_bytes};
use std::path::Path;

fn bench_int_loop(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own i: Int64 = 0;
        let own n: Int64 = 1000000;
        while i < n {
            i = i + 1;
        }
        return i;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("int_loop_1m", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_int_loop_10m(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own i: Int64 = 0;
        let own n: Int64 = 10000000;
        while i < n {
            i = i + 1;
        }
        return i;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("int_loop_10m", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_array_push_pop(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own arr: Arr<Int64> = vec_new();
        let own i: Int64 = 0;
        let own n: Int64 = 100000;
        while i < n {
            arr = vec_push(arr, i);
            i = i + 1;
        }
        let own len: Int64 = vec_len(arr);
        let own j: Int64 = 0;
        while j < len {
            let own _v: Int64 = vec_pop(arr);
            j = j + 1;
        }
        return len;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("array_push_pop_100k", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_map_insert_lookup(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own m: Map<Str, Int64> = map_new();
        let own i: Int64 = 0;
        let own n: Int64 = 100000;
        while i < n {
            m = map_insert(m, str_concat(\"k\", int_to_str(i)), i);
            i = i + 1;
        }
        let own sum: Int64 = 0;
        let own j: Int64 = 0;
        while j < n {
            sum = sum + map_get(m, str_concat(\"k\", int_to_str(j)));
            j = j + 1;
        }
        return sum;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("map_insert_lookup_100k", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_procedure_call(c: &mut Criterion) {
    let source = "pub proc add(a: Int64, b: Int64) -> Int64 {
        return a + b;
    }
    pub proc main() -> Int64 {
        let own i: Int64 = 0;
        let own n: Int64 = 1000000;
        let own sum: Int64 = 0;
        while i < n {
            sum = sum + add(i, i);
            i = i + 1;
        }
        return sum;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("procedure_call_1m", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_string_concat(c: &mut Criterion) {
    let source = "pub proc main() -> Str {
        let own s: Str = \"\";
        let own i: Int64 = 0;
        let own n: Int64 = 10000;
        while i < n {
            s = str_concat(s, \"x\");
            i = i + 1;
        }
        return s;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("string_concat_10k", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_nested_if(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own i: Int64 = 0;
        let own n: Int64 = 1000000;
        let own sum: Int64 = 0;
        while i < n {
            if i < 100 {
                sum = sum + 1;
            } elif i < 200 {
                sum = sum + 2;
            } elif i < 300 {
                sum = sum + 3;
            } elif i < 400 {
                sum = sum + 4;
            } elif i < 500 {
                sum = sum + 5;
            } elif i < 600 {
                sum = sum + 6;
            } elif i < 700 {
                sum = sum + 7;
            } elif i < 800 {
                sum = sum + 8;
            } elif i < 900 {
                sum = sum + 9;
            } else {
                sum = sum + 10;
            }
            i = i + 1;
        }
        return sum;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("nested_if_1m", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_json_load(c: &mut Criterion) {
    let source = "pub proc main() -> Int64 {
        let own i: Int64 = 0;
        let own n: Int64 = 100000;
        let own sum: Int64 = 0;
        while i < n {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
    }";
    let bytes = compile_to_bytes_with_options(source, Path::new(""), CompileOptions::default()).unwrap();
    c.bench_function("json_load_100k", |b| {
        b.iter(|| {
            let _ = load_executor_from_bytes(black_box(&bytes)).unwrap();
        })
    });
}

fn bench_deep_expr(c: &mut Criterion) {
    let mut expr = "1".to_string();
    for i in 2..=100 {
        expr = format!("({} + {})", expr, i);
    }
    let source = format!("pub proc main() -> Int64 {{\n    return {};\n}}", expr);
    let executor = build_executor(&source).unwrap();
    c.bench_function("deep_expr_100", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

fn bench_string_ops(c: &mut Criterion) {
    let source = "pub proc main() -> Str {
        let own s: Str = \"hello\";
        let own i: Int64 = 0;
        let own n: Int64 = 100000;
        while i < n {
            s = str_concat(s, \"x\");
            i = i + 1;
        }
        return s;
    }";
    let executor = build_executor(source).unwrap();
    c.bench_function("string_ops_100k", |b| {
        b.iter(|| {
            let _ = executor.run("main").unwrap();
        })
    });
}

criterion_group!(
    vm_benches,
    bench_int_loop,
    bench_int_loop_10m,
    bench_array_push_pop,
    bench_map_insert_lookup,
    bench_procedure_call,
    bench_string_concat,
    bench_nested_if,
    bench_json_load,
    bench_deep_expr,
    bench_string_ops,
);

criterion_main!(vm_benches);
