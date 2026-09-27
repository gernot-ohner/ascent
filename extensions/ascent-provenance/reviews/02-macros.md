# 2. External macros

This PR adds `provenance!` and `provenance_run!` on top of the values in the
first PR. It contains the parser, local macro expansion, lowering, wrappers,
compatibility tests and diamond example. Stock Ascent still evaluates the rules.

Read these **261 lines**, the macro portion of the overall 499-line reading
list. Links point to this layer's branch.

| Read | Lines | Check |
| --- | ---: | --- |
| [lower.rs:13-155](https://github.com/gernot-ohner/ascent/blob/codex/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L13-L155) and [173-184](https://github.com/gernot-ohner/ascent/blob/codex/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L173-L184) | 155 | Tracked premises contribute a product; tracked heads receive it as a new column. |
| [normalize.rs:5-32](https://github.com/gernot-ohner/ascent/blob/codex/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/normalize.rs#L5-L32) | 28 | Duplicate keys join their witnesses; zero rows disappear before indexing. |
| [wrapper.rs:103-180](https://github.com/gernot-ohner/ascent/blob/codex/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/wrapper.rs#L103-L180) | 78 | Named programs retain the engine; inline programs execute in the caller's scope. |
| **Total** | **261** | |

The parser and macro-hygiene code form most of this diff. They stay together:
lowering consumes expanded syntax, and the wrapper must preserve Rust scope.
Splitting those pieces would leave intermediate PRs without a usable macro.
[Source attribution](../THIRD_PARTY_NOTICES.md) identifies the extracted code.
The earlier wrapper helpers handle names, `Self` and initializer order; they
are outside the short reading list.

Check rules with several tracked premises, several rules for one head, and
bodies with no tracked premises. The last case contributes one, the empty
witness. Ordinary relations remain fixed background; passing through one loses
upstream annotations. Tracked negation, aggregation and custom storage are
rejected.

Named programs normalize at `Default` and before the first run. An unchanged
rerun reuses the engine. After changing relation storage, or after a timeout or
panic, use a fresh instance. Inline results are owned and run once. Why mode
requires acyclic derivations; Boolean recursion supports finite reachable data
under the documented monotonicity assumptions.

Run from `extensions/ascent-provenance/`:

```sh
cargo +1.85.0 test --workspace --locked
python3 tests/inline_capture.py
python3 tests/compile_fail.py
cargo +1.85.0 run --manifest-path tests/consumer/Cargo.toml --features parallel-stock --locked
python3 tests/relocated_consumer.py
```

Expect 58 workspace tests, seven diagnostic checks and passing consumer runs.
The feature regressions live here, including the `Self` fixes from PR #4.
The final PR adds the independent oracle and performance measurements.

[Overall stack guide](https://github.com/gernot-ohner/ascent/blob/codex/provenance-validation/extensions/ascent-provenance/REVIEW_GUIDE.md)
