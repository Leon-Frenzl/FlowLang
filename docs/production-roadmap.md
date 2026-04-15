# FlowLang Production Roadmap

This roadmap defines the path from prototype to production-grade language/runtime.

## Production Definition

FlowLang is production-ready when it meets all of the following:

- Language semantics are versioned and backward-compatibility policy is documented.
- Compiler and verifier are deterministic and stable across supported platforms.
- Runtime behavior is observable (metrics, logs, error codes) and testable.
- Safety guarantees (linearity + contract integrity) are enforced by default.
- Release process, CI quality gates, and support policy are documented and automated.

## Phase A: Language Stabilization

- Freeze syntax for v0.1 and document unsupported constructs explicitly.
- Define formal semantics for:
  - linear value ownership transfer
  - transition/violation path completeness
  - shared_contract mutation constraints
- Add parser/verifier golden tests for every syntax form in docs/grammar.ebnf.

Exit criteria:

- Every grammar production has positive and negative tests.
- No verifier nondeterminism in repeated runs.

## Phase B: Compiler Architecture Hardening

- Split `src/main.rs` into modules:
  - parser
  - ast
  - verifier
  - bytecode
  - native
  - cli
- Introduce internal IR with explicit control-flow graph and typed values.
- Add stable diagnostic IDs for parser/verifier/runtime errors.

Exit criteria:

- Module-level tests exist for each subsystem.
- Error diagnostics include machine-readable ID + source span.

## Phase C: Runtime and JIT Reliability

- Expand native backend from constant arithmetic to data-driven expressions.
- Add deterministic runtime simulation mode for protocol event replay.
- Add runtime validation pass before JIT emission as a hard gate.

Exit criteria:

- Native backend cross-checks results with interpreter for supported subset.
- Replay tests cover protocol happy and failure paths.

## Phase D: Developer Experience

- Add language server basics (syntax + diagnostics).
- Add formatter rules and style guide.
- Provide starter templates for:
  - TCP handshake
  - payload relay
  - chat relay app

Exit criteria:

- New users can scaffold, validate, and run examples in < 5 minutes.

## Phase E: Release Engineering

- Semantic versioning and changelog process.
- Tagged releases with binary artifacts.
- Security policy and vulnerability disclosure process.

Exit criteria:

- Repeatable release from CI with signed tags and release notes.

## Non-Negotiable Safety Gates

The following must remain mandatory in all release builds:

- No linear value may be used after move.
- No linear value may leak across any path.
- No contract handler may have an unterminated control-flow path.
- No transition may target an undefined state.
- shared_contract handlers cannot transition.
