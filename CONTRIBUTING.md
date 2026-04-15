# Contributing to FlowLang

## Development Commands

- `cargo test` - run unit tests
- `cargo fmt --all` - format code
- `cargo clippy --all-targets -- -D warnings` - lint with warnings denied
- `cargo run -- check <file.flow>` - parse and verify a FlowLang file

## Coding Standards

- Keep changes minimal and targeted.
- Preserve safety checks; do not weaken verifier invariants.
- Prefer explicit names for states, handlers, and linear resources.
- Add tests for each bug fix and each new verifier rule.

## Safety Invariants (Must Not Be Broken)

- Linear resources are consumed exactly once.
- All protocol handler paths terminate through transition or violation.
- Transitions only target declared states.
- shared_contract handlers do not perform state transitions.

## Pull Request Requirements

- CI must pass (fmt, test, clippy).
- Include at least one test for behavior changes.
- Update docs when syntax, verifier rules, or runtime behavior changes.

## Commit Style

Use short, action-oriented commit messages, for example:

- `verifier: enforce transition target validation`
- `native: add branch codegen for comparison conditions`
- `docs: clarify contract integrity constraints`
