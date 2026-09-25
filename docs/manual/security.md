<!-- description: What a reflex can do when it runs as you, what evoke guarantees around installing and deciding, and what it does not guarantee. There is no sandbox yet. -->
# Security

A fetched reflex runs as you. `evoke add` and `evoke update` are the trust decisions. Everything below is what
the tool guarantees around them, and what it does not.

## What runs, and as whom

- A body runs with your user, under a scrubbed environment of five variables: `PATH`, `HOME`, `TMPDIR`, `LANG`,
  `TERM`. It receives secrets only through its config, for one run. The scrubbing prevents leaks. It does not
  contain malice. **Read what you install.** A reflex handed to the SDK as code is your application's own. It
  runs in its process, unscrubbed. It must honour its `signal`, since nothing can end it from outside.
- No code runs at install time. `add` fetches a tree, reads a manifest, lints it, and writes files. A body runs
  only at a decision, at `evoke run`, or when `evoke check` imports it to see that it exports a function.
- There is no sandbox yet. Containment, with Landlock on Linux and Seatbelt on macOS, comes together with a
  permissions field in the manifest, in one change. So a claim and its enforcement arrive together.

## What the classifier cannot do

- **State is only `{ request }`.** The adapter sees the sentence and the questions built from installed manifests.
  Nothing else of yours.
- **Arguments are closed sets or exact spans.** A value is an author's option key, one of your vocabulary words,
  or a piece of the input read by a recognizer and checked against its range. Injected text can choose a call. It
  can never mint a value.
- **The effect gate covers what it chooses.** A destructive reflex always confirms, and there is no `--yes`.
  Unattended use is a threshold in a file you own and trust. A `read` or `write` reflex over its floor runs
  without asking. Whatever it does with a span it is handed, like a URL or a quoted text, an injected sentence
  can make it do. So its effect is the author's promise about exactly that.
- **Prompts cannot be repainted.** Control characters and bidi overrides are refused in every manifest, overlay
  and vocabulary string, and in every span. `evoke`'s own line, with the call, the effect and the weakest
  judgment, prints before the reflex's template.
- **An input over 2 000 characters is refused**, and one decision has 30 seconds.

## What a manifest cannot do

- **Wording is linted and conflict-tested at `add`.** Text that addresses a model is named. Every phrase a
  newcomer would steal from an installed reflex is reported, with its one-line fix.
- **Effect only tightens.** The lock holds the effect you consented to. Upstream may tighten it. Loosening waits
  for `evoke update --accept`. Your overlay may tighten it too, never loosen it.
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
`mise run test` runs them without a key.

## Reporting

Report a vulnerability privately, through
[GitHub's report form](https://github.com/evoke-build/evoke/security/advisories/new), never in a public issue.
