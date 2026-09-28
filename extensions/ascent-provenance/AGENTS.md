# Implementation guardrails

- Keep Ascent itself unmodified. Use registry Ascent 0.8.1; no path/git override.
- Extract frontend parsing/expansion only; never copy evaluation or index planning.
- Preserve the original experiment checkout.
- Read and inspect each verification result before scheduling its commit.
  Do not queue a commit in the same tool batch that polls an unfinished test.
- Source-inclusion boundaries must use parser cursor identity, not token-span
  equality: generated tokens can share spans. Keep the inclusion regression test.
- Macro-hygiene unit fixtures need distinct source spans; also compile real
  downstream macro invocations so string round trips cannot hide capture bugs.
- Inline rule-body captures must remain supported through stock `ascent_run!`.
  Inline results execute once; reruns and timeouts belong to named programs.
  Keep `tests/inline_capture.py` and the inline execution regressions passing.
- Named Self rewriting must preserve nested Rust items' own Self scope. Opaque
  Rust macro syntax requires an explicit program type; never blindly replace
  tokens across arbitrary macro or item boundaries.
