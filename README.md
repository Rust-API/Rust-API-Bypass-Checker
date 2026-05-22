# Rust API Bypass Checker

A focused MIR-based static analysis artifact for detecting locally provable Rust safe-to-unsafe API replacement opportunities.

The checker reasons over local MIR facts such as integer intervals and pointer nullness to conservatively report cases where checked APIs, such as `slice.get(i)` or `split_at(mid)`, may be replaced by unchecked counterparts under provable preconditions.

## What This Demo Claims

This repository is being maintained as a focused demo artifact, not as a whole-program optimizer.

The intended claim is narrow:

> For a known set of standard-library API pairs, the analyzer can prove selected call-site preconditions from local MIR facts and report replacement opportunities.

The demo does not claim:

- whole-program optimization
- whole-application speedup
- complete alias analysis
- full raw-pointer reasoning
- semantic preservation for arbitrary Rust programs

Unsupported calls and memory effects are conservatively downgraded to local unknown, and replacement candidates are suppressed when their proof would depend on those unknown facts.

## Supported API Fragment

The analyzer is intentionally narrow.

- The active numerical domain is `interval`.
- Reasoning is local and MIR-based.
- Default descent into ordinary callees is disabled.
- Special handling is limited to a small whitelist of checked/unchecked API families.
- Calls outside the supported fragment are downgraded to local unknown at call boundaries.

The primary demo matrix is:

| API family | Required condition | Demo case |
|---|---|---|
| `checked_add` | result is within the integer type range | `tests/checked_add` |
| `slice::get` / `get_mut` | `index < slice.len()` | `tests/get` |
| `slice::split_at` / `split_at_mut` | `mid <= slice.len()` | `tests/split_at`, `tests/ring_buffer_split` |
| `slice::swap` | `i < slice.len() && j < slice.len()` | `tests/swap` |
| `ptr.as_ref()`, `ptr.as_mut()`, `NonNull::new(ptr)` | pointer is locally known non-null | internal nullness support |

See [DEMO_SCOPE.md](DEMO_SCOPE.md) for the detailed demo boundary and expected output shape.

## Example

```rust
fn process_array(arr: &[i32]) {
    for i in 0..arr.len() {
        // Redundant bounds check - loop condition guarantees i < arr.len()
        let value = arr.get(i).unwrap(); // Can optimize to arr[i] or arr.get_unchecked(i)
        println!("Value is {}", value);
    }
}
```

Within its supported fragment, the checker aims to recognize that `i < arr.len()` already holds and report the call as a replacement candidate for an unchecked variant. It reports diagnostics only; it does not rewrite source code automatically.

## Repository Layout

- `src/`: analyzer implementation and command-line drivers
- `tests/`: small standalone demo crates used for focused experiments
- `analysis-results/`: saved experiment outputs
- `evaluation/`: scripts and notes for measurement runs
- `API-counterprats/`: benchmark-oriented companion workspace
- `unchecked_method_base/`: safe/unsafe counterpart inventory material

## Requirements

- Rust nightly `nightly-2025-01-10`
- `rustc-dev`
- `llvm-tools-preview`
- system libraries for `gmp`, `mpfr`, and `z3`

Example setup on Ubuntu:

```sh
rustup toolchain install nightly-2025-01-10
rustup component add rustc-dev llvm-tools-preview --toolchain nightly-2025-01-10
sudo apt-get install libgmp-dev libmpfr-dev libz3-dev
```

The root crate pins its toolchain in [rust-toolchain.toml](rust-toolchain.toml). The benchmark workspace under `API-counterprats/` may use a different nightly for separate experiments.

## Build

```sh
$ git clone https://github.com/Rust-API/Rust-API-Bypass-Checker.git
$ cd Rust-API-Bypass-Checker
$ export RUSTFLAGS="-Clink-args=-fuse-ld=lld"
$ cargo build
```

This builds the main analyzer binary `api-bypass` together with the `cargo-api-bypass` wrapper.

## Usage

### Direct Analyzer Binary

The most reproducible way to run the demo is to point `api-bypass` at one of the standalone source files in `tests/`.

1. Inspect candidate entry functions:

```sh
mkdir -p target/demo-smoke
./target/debug/api-bypass tests/get/src/main.rs --show_reachable_entries --emit=metadata --out-dir target/demo-smoke
```

2. Analyze a specific entry:

```sh
./target/debug/api-bypass tests/get/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
```

Useful flags:

- `--entry_def_id_index <n>`: analyze the selected DefId index
- `--show_all_entries`: print all candidate entries discovered in the current crate
- `--show_reachable_entries`: print reachable entry candidates from the current front-end scan
- `--auto_analysis`: let the checker pick an entry automatically when supported
- `--domain interval`: explicitly select the interval domain

### Cargo Wrapper

The repository also provides a `cargo api-bypass` wrapper binary:

```sh
cargo run --bin cargo-api-bypass -- api-bypass --help
```

Its current interface is minimal and is mainly useful when experimenting with Cargo-driven entry flows.

## Demo Smoke Commands

These commands match the current small demo crates and keep outputs isolated under `target/demo-smoke`:

```sh
mkdir -p target/demo-smoke
env RUST_LOG=warn ./target/debug/api-bypass tests/checked_add/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
env RUST_LOG=warn ./target/debug/api-bypass tests/get/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
env RUST_LOG=warn ./target/debug/api-bypass tests/split_at/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
env RUST_LOG=warn ./target/debug/api-bypass tests/swap/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
env RUST_LOG=warn ./target/debug/api-bypass tests/ring_buffer_split/src/main.rs --entry_def_id_index 3 --emit=metadata --out-dir target/demo-smoke
```

Expected current behavior:

| Command target | Expected candidate output |
|---|---|
| `tests/checked_add` | one `integer.checked_add(rhs)` candidate |
| `tests/get` | one `slice::get(index)` candidate |
| `tests/split_at` | one `slice::split_at(mid)` candidate |
| `tests/swap` | one `slice::swap(i, j)` candidate |
| `tests/ring_buffer_split` | two `slice::split_at_mut(mid)` candidates, plus conservative call-boundary warnings in the loop body |

## Result Semantics

Diagnostics should be interpreted conservatively.

- A supported diagnostic comes from the currently supported numerical or pointer-nullness fragment.
- A call-boundary or unsupported diagnostic means the analyzer deliberately stopped and downgraded the result to unknown.
- The absence of a diagnostic is not a global proof of safety.

When a supported safe API call is proven to satisfy the corresponding unchecked precondition, the analyzer emits a replacement-candidate diagnostic shaped like:

```text
[Bypasser] Replacement candidate

Safe API:
  slice::split_at(mid)

Suggested replacement:
  unsafe { slice::split_at_unchecked(mid) }

Required condition:
  mid <= slice.len()

Analysis result:
  proven from local MIR split-index facts
```

## Related Files

- [DEMO_SCOPE.md](DEMO_SCOPE.md): demo boundary, output contract, and smoke commands
- [E2E_CASE_STUDY_EXPERIMENTS.md](E2E_CASE_STUDY_EXPERIMENTS.md): experiment notes
- [evaluation/README.md](evaluation/README.md): evaluation workflow
- [API-counterprats/README.md](API-counterprats/README.md): benchmark companion workspace

## License

See [LICENSE](LICENSE) and [licenses](licenses).

## Acknowledgments

Built upon:

- [MirChecker](https://github.com/lizhuohua/rust-mir-checker)
- [MIRAI](https://github.com/facebookexperimental/MIRAI)
- [Crab](https://github.com/seahorn/crab)
