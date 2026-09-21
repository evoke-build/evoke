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
  owned paths is checked at every run. A `git pull` that changed an overlay is a stop, not a surprise.
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
  So a cloned repository cannot point your decisions at a classifier of its choosing.
- **The SDK never searches.** `load` takes an explicit root, never fetches, and never writes. It hands each
  tenant their own vocabulary. So a server cannot be hijacked by whatever project a working directory holds.

## Reporting

Report a vulnerability privately to the maintainers at [evoke-build/evoke](https://github.com/evoke-build/evoke)
rather than in a public issue.
