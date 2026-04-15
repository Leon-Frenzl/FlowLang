# Architektur - Phase 1

## Pipeline

1. Lexer: Wandelt Quelltext in Token-Stream um.
2. Parser: Erstellt AST fuer Vertraege, Zustaende und Handler.
3. Linearity-Checker: Prueft lineare Ressourcen pro Ausfuehrungspfad.
4. Bytecode-Compiler: Uebersetzt Handler in eine Instruktionsfolge.
5. VM: Fuehrt Bytecode aus (als Vorstufe einer spaeteren JIT-Engine).

## Linearitaetsmodell

- Parameter mit Typnamen, die mit `Linear` beginnen, gelten als linear.
- Ein linearer Wert darf nur einmal konsumiert werden.
- Konsumierende Operationen in Phase 1:
  - Funktionsaufrufargumente (`create_syn_ack(p)`)
  - `send x;`
  - `drop x;`
- Der Checker simuliert alle Kontrollfluss-Pfade.
- Am Ende eines Handlers muessen alle linearen Werte konsumiert sein.

## Explizites Violation-Handling

Fuer Handler mit linearen Eingaben erzwingt der Verifier in Phase 1:

- Es muss mindestens ein `violation { ... }`-Block vorhanden sein.
- Jedes `if` braucht ein `else`, damit kein Eingabepfad unbehandelt bleibt.
- Der `else`-Pfad muss Violation-Handling enthalten (direkt oder verschachtelt).

Verletzungen dieser Regeln fuehren zu einem Compile-Error.

## Contract-Integritaet (Unbreakable Contracts)

Der Verifier erzwingt zusaetzlich strukturelle Vertragsregeln:

- Keine doppelten State-Namen innerhalb eines Contracts.
- Keine doppelten Handler-Namen innerhalb eines States.
- Jeder State in `contract` muss mindestens einen Handler besitzen.
- Jede `transition -> TARGET;` muss auf einen existierenden State zeigen.
- `shared_contract`-Handler duerfen keine Transition ausfuehren.
- In `contract`-Handlern muss jeder Kontrollfluss-Pfad mit `transition` oder `violation` terminieren.

Damit sind ungueltige Zustandsautomaten zur Compile-Zeit ausgeschlossen.

## Bytecode-Modell

Die VM arbeitet mit einer kleinen Instruktionsmenge:

- `Eval(expr)`
- `Call(target)`
- `Send(expr)`
- `Drop(expr)`
- `JumpIfFalse(cond, target)`
- `Jump(target)`
- `Transition(state)`
- `Nop`

## JIT-Roadmap

Phase 1 erzeugt bewusst Bytecode und fuehrt diesen in einer VM aus.
Damit ist die Trennschicht fuer spaetere native Backends klar:

- Bytecode -> x86_64 Codegen
- Bytecode -> ARM64 Codegen
- Runtime-Verifikation bleibt als Gate vor nativer Kompilierung
