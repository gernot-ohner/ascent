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
- Inline rule-body captures are an unresolved compatibility gate. Do not call
  the migration complete or silently remove the gate without resolving the
  design decision recorded in VERIFICATION.md.
