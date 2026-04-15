# FlowLang (JIT-Edition) - Phase 1 Prototype

FlowLang ist eine protokoll-native Sprache mit linearer Ressourcen-Semantik.
Dieses Repository implementiert den **Phase-1 Kern** als lauffaehigen Prototyp:

- Tokenizer + Parser fuer eine erste DSL
- AST-Modell fuer `shared_contract`, `contract`, `state`, `on`
- Linearity-Checker (Use-after-move und Leak pro Pfad)
- Bytecode-Compiler + kleine VM als JIT-Basis
- TCP-Three-Way-Handshake-PoC als Beispiel

## Warum Rust fuer den Prototyp?

- Low-Level-nahe Runtime-Modelle ohne GC
- Gute Basis fuer spaetere JIT-Backends (x86_64, ARM)
- Sichere Entwicklung des Compilers selbst

## Quickstart

```bash
cargo run -- check examples/tcp_handshake.flow
cargo run -- compile examples/tcp_handshake.flow
cargo run -- run examples/tcp_handshake.flow
cargo run -- jit-run examples/jit_const.flow
cargo run -- jit-run examples/jit_branch.flow
```

## CLI

- `check <file>`: Parsen + formale lineare Verifikation
- `compile <file>`: Parsen + Check + Bytecode-Dump
- `run <file>`: Parsen + Check + Bytecode in VM ausfuehren
- `jit-run <file>`: Native x86_64-Demo, die konstante Add-Ausdruecke als echte Register-Instruktionen emittiert und ausfuehrt

## Phase-1 Scope

Der Prototyp fokussiert auf:

- Linear Types als Ressourcenfluss
- Explizite Violation-Pfade
- Verifier-Regeln fuer lineare Handler:
	- `if` muss mit `else` abgeschlossen sein
	- `else` muss Violation-Handling besitzen
- Zustandshandler und Transitions

Nicht enthalten (noch):

- Echter Maschinen-JIT
- Vollstaendiges Typsystem
- Optimierungen wie SSA oder Register Allocation

Siehe [docs/grammar.ebnf](docs/grammar.ebnf) und [docs/architecture.md](docs/architecture.md).
