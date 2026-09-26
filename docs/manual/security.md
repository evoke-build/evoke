<!-- description: What a reflex can touch when it runs as you, how the kernel holds it to its declaration, and what evoke guarantees around installing and deciding. -->
# Security

A fetched reflex runs as you, held to what its manifest declares. `evoke add` and `evoke update` are the trust
decisions. Everything below is what the tool guarantees around them, and what it does not.

## What runs, and as whom

- A body runs with your user, under a scrubbed environment of five variables: `PATH`, `HOME`, `TMPDIR`, `LANG`,
  `TERM`. `TMPDIR` is a private folder, made for the run and removed after it. The body receives secrets only
  through its config, for one run. The scrubbing prevents leaks. **Read what you install**, and its declaration,
  the next point, with it.
- **A body is held to its declaration.** `[needs]` in the manifest names the paths the body reads and writes,
  whether it reaches the network, and the programs it runs:
  [What the body touches](author/manifest.md#what-the-body-touches). Leaving the table out is the tightest
  declaration: the body's own directory, `TMPDIR`, and nothing else. Two layers hold the body there. The
  kernel's: on Linux a Landlock ruleset, `no_new_privs` and a seccomp filter that closes the network, applied to
  the process before the body starts; on macOS Seatbelt, through `sandbox-exec`, with a profile that denies
  everything the declaration does not name. And, for a file body, Node's permission model, which refuses a read
  past the declaration with the path before the kernel is asked. A program named in `runs` runs under the
  kernel's layer too, so it reaches no further than the body; what it asks the system to do — Finder emptying
  the trash, the login window restarting — happens outside the declaration, so `runs osascript` is read beside
  the effect. `evoke check` imports a body under the same hold, with nothing configured.
- **A reach past the declaration ends the run.** The line names what was reached and the key: `~/secret.txt is
  not in [needs] reads`, `example.com is not in [needs] hosts`, `[needs] runs names no program`. Its fix depends
  on where the declaration is written: the manifest's line for a local reflex; for a fetched one, `evoke update
  --accept` when upstream's own declaration already allows it, else `evoke remove`. The run exits 1, a failure.
- **The status is printed, never assumed.** A machine that holds only part of a declaration, or none of it — a
  Linux kernel without Landlock or before its third version, a Mac without `sandbox-exec` — still runs the body,
  and says so: one line at `add` and at `show`, `  not contained  <why>`; ` · partly contained` or ` · not
  contained` at the end of the line `evoke` prints before every body runs; and `contained` in `--json`.
- No code runs at install time. `add` fetches a tree, reads a manifest, lints it, and writes files. A body runs
  only at a decision, at `evoke run`, or when `evoke check` imports it to see that it exports a function.
- A reflex handed to the SDK as code is your application's own. It runs in its process, unscrubbed and unheld.
  It must honour its `signal`, since nothing can end it from outside. A reflex the SDK loads from a project runs
  held as far as the package can: under Node's permission model on both systems, and under Seatbelt on macOS.
  On Linux the kernel's layer needs native code the package does not carry, so a file body runs partly held and
  an argv body unheld there, and the result's `contained` says which.

## What the classifier cannot do

- **State is only `{ request }`.** The adapter sees the sentence and the questions built from installed manifests.
  Nothing else of yours.
- **Arguments are closed sets, exact spans, or an earlier step's result.** A value is an author's option key, one
  of your vocabulary words, a piece of the input read by a recognizer and checked against its range, or, in a
  sentence of several steps, a field of an earlier step's result or that step's whole result, bound by the plan
  and shown on its line before anything runs. Injected text can choose a call. It can never mint a value.
- **The effect gate covers what it chooses.** A destructive reflex always confirms, and there is no `--yes`.
  Unattended use is a threshold in a file you own and trust. A `read` or `write` reflex over its floor runs
  without asking. Whatever it does with a span it is handed, like a URL or a quoted text, an injected sentence
  can make it do. So its effect is the author's promise about exactly that. A `write` reflex that takes an
  earlier step's whole result runs with it over its floor, unasked; the plan prints `takes` on its line first,
  and a destructive reflex takes none.
- **Prompts cannot be repainted.** Control characters and bidi overrides are refused in every manifest, overlay
  and vocabulary string, and in every span. `evoke`'s own line, with the call, the effect and the weakest
  judgment, prints before the reflex's template.
- **An input over 2 000 characters is refused**, and one decision has 30 seconds.

## What a manifest cannot do

- **Wording is linted and conflict-tested at `add`.** Text that addresses a model is named. Every phrase a
  newcomer would steal from an installed reflex is reported, with its one-line fix.
- **Effect and needs only tighten.** The lock holds the effect and the declaration you consented to. Upstream
  may tighten either at any tag. Loosening waits for `evoke update --accept`, which says what widened. Your
  overlay may tighten the effect, never loosen it, and cannot touch `[needs]`.
- **The contract cannot be overridden.** An overlay that tries makes the reflex inactive, not looser.
- **Tags are only for `--tag`.** No label, category or claim in a manifest changes what may run.

## What the files guarantee

- **Trust binds to content.** A project outside home decides only after `evoke trust`. The digest of its four
  owned paths is checked at every run. A `git pull` that changed an overlay is a stop, not a surprise. A local
  reflex, `./dir`, is outside those paths. Its directory is yours, so a pull that changes it runs what it now
  holds, under the effect its manifest now claims.
- **The lock binds to content.** Every remote reflex is pinned by tag, commit and `h1`. The store's copy is
  hashed again every time `evoke` starts. A tag that moved is refused.
- **Fetching is narrow.** `git` is called with a strict ref grammar, `--end-of-options`, and an allow-list of
  `https` and `ssh`. Symlinks and submodules are rejected. There is no checkout, no hook, and no clone-time
  script.
- **Secrets are referenced, never stored.** `evoke.toml` names a variable. The value exists in your environment,
  and reaches a body for the length of one run. `evoke` writes no key and no config value under the cache or the
  state directory. The log and the REPL's history hold what you typed and what a body returned. Only you can read
  them.
- **Adapter names resolve on your machine only**, against the tool's built-ins, never from a project directory.
  So a cloned repository cannot point your decisions at a classifier of its choosing. The endpoint is built into
  the adapter, whose id keys the cache and the lock, so no file and no variable moves it. A proxy named in your
  environment carries the connection there and sees ciphertext: the CLI trusts Mozilla's roots, never your
  platform's, and the SDK trusts what your Node trusts.
- **The SDK never searches.** `load` takes an explicit root, never fetches, and never writes. It hands each
  tenant their own vocabulary. So a server cannot be hijacked by whatever project a working directory holds.

## Check it yourself

Nothing on this page needs a key to test. `evoke run <call>` runs a reflex by name and calls no classifier. The
`replay` adapter answers every decision from a file you write: set `adapter = "replay"` in `evoke.toml` and name
the file in `EVOKE_ANSWERS`. [Testing](sdk/testing.md#the-cli-on-a-recording) shows the file. An answer that
tries to mint a value, a key no argument offers or a number that is not a probability, ends the decision with a
fault, and nothing runs. The repository's `spec/` holds the transcripts and vectors behind the claims above, and
`mise run test` runs them without a key. The `contained` transcript shows a body refused past its declaration,
and a declared path the machine lacks refused before the body runs.

## Reporting

Report a vulnerability privately, through
[GitHub's report form](https://github.com/evoke-build/evoke/security/advisories/new), never in a public issue.
