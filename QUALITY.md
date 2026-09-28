# Quality

`evoke` decides which program runs on your machine, and with which values. So it is built to an exceptional
standard, defined by three rules. This page shows how each rule is kept, and how to check them yourself.

## Every detail is measured

What `evoke` must do is written down as examples a test can check, and it is built until they pass.

| What         | How it is checked                                                                                               |
| :----------- | :-------------------------------------------------------------------------------------------------------------- |
| Output       | Every recorded session in [spec/transcripts](spec/transcripts) is replayed against the built program, offline, and each line must match |
| Decisions    | Each of the core's public operations has its cases in [spec/vectors](spec/vectors), run by the Rust tests and again through the TypeScript SDK |
| Guarantees   | [Property tests](crates/evoke-core/tests) generate inputs no one wrote, and check what must always hold: a step that cannot be undone never runs without a yes, and a change never runs beside another step. Each failure is shrunk to its smallest case |
| Parsing      | The manifest reader, the reading of a sentence and the four value recognizers are [fuzzed](fuzz): fed random variations of real inputs, none may crash, and a date, a time, an amount or a code found in a sentence must read again alone at a prompt |
| Confidence   | [`evoke calibrate`](https://evoke.build/manual/use/calibrating.html) checks each confidence against your own examples and tests: a call given 0.85 should be right about 85 times in 100 |
| Collection   | Each reflex under [reflexes/](reflexes/README.md) is checked as it stands in the repository: its manifest reads without a warning, and its program loads within the limits it declares |
| Pages        | The manual and the site are [built](mise-tasks/site) on every change, and each link between their pages must land on a page and an anchor that exist |
| Code         | Rust code is formatted, and the workspace is held to the pedantic lints of clippy, Rust's linter, with every warning an error |
| Arithmetic   | The release build keeps [overflow checks](Cargo.toml), so an overflow is reported as a bug instead of becoming a wrong number |
| Dependencies | Each one the workspace uses has its reason [written beside it](Cargo.toml), and [an audit](deny.toml) checks their licences, known vulnerabilities and sources. The SDK depends on nothing at run time |
| Tools        | Each is pinned to an exact version in [mise.toml](mise.toml)                                                    |

CI runs lint and the tests on every change to `main` and on every pull request. It runs the tests again under the
oldest and the newest Node the SDK supports. On macOS it replays the sessions and checks the collection again,
since a different kernel confines each reflex there. Fuzzing runs outside CI, ten minutes for each target. A
release is built only from a tree that passes. It publishes checksums for its archives, GitHub keeps a signed
record of how they were built, and the SDK reaches npm with its provenance.

## Every boundary is clear

- **The core is pure.** `evoke-core` makes every decision. `evoke-adapters` turns its questions into requests to
  the classifier, and the replies into answers. Neither touches a file, a clock, the network or a process. The
  command line and the SDK hand them values, and do what they return.
- **Dependencies point one way.** The command line and the SDK's WebAssembly build use the adapters, and the
  adapters use the core. Nothing points back.
- **One core under the CLI and the SDK.** The SDK runs the same core, compiled to WebAssembly, and makes no
  decision of its own. The same cases check both.
- **Unsafe code stays at the edge.** Rust's `unsafe` is forbidden in the core and the adapters. It is allowed only
  in the modules that call the kernel directly, and at the SDK's WebAssembly boundary.
- **A reflex is held to what it declares.** Its manifest names the paths it reads and writes, whether it reaches
  the network, and the programs it runs. On the command line, the kernel enforces those limits. Where a machine
  cannot, `evoke` says so before the reflex runs. [Security](https://evoke.build/manual/security.html) shows how
  far the SDK can hold one.

## Nothing is added on a hunch

- **A study comes first.** Before a feature is built, a study measures the problem, weighs the options and writes
  down the design. The feature is then built to that design, and released.
- **Each feature names its bar.** The number it must reach, or the promise it must keep, is written down before
  work starts. It ships with the check that shows it holds: a recorded session, a set of cases, a property test or
  a fuzz target.
- **Each feature is reviewed.** Its code is read for correctness, and its words for a reader new to `evoke`. Every
  page it touches is checked at a phone's width and a desk's. Every finding is fixed before it ships.
- **A dependency earns its place.** One is added only when writing the same code by hand would be worse.
- **A change is carried through.** Every caller, test and page it affects changes with it, and what it makes
  obsolete is removed.

## Check it yourself

The checks run on a laptop as they run in CI, and none needs a key to the classifier: `mise run lint`, then
`mise run test`. `mise run fuzz` fuzzes each target for ten minutes. [Developing](README.md#developing) sets up the
tools.
