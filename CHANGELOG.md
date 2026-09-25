# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- A body that runs a program no longer prints Node's warning about `--allow-child-process`.
- Under Node 25 and later, a body that declares a host reaches the network, and Node's own layer keeps a body
  that declares none off it.
- A network reach past the declaration is named on every Node, a listening port and a datagram included. An error
  a body throws from a callback ends the run with its message.

## [0.5.0] - 2026-09-25

- A manifest declares what its body touches, under `[needs]`: the paths it reads and writes, whether it reaches
  the network, the programs it runs; leaving the table out is the tightest declaration. The kernel holds the body there, through
  Landlock and seccomp on Linux and Seatbelt on macOS, with Node's permission model under both; a reach past the
  declaration ends the run with the path and the key, and the manifest's line as the fix. `add` and `show` print
  each declaration under its row, and say once when a machine holds only part of one. The lock records the
  declaration you consented to; upstream may narrow it, and widening waits for `evoke update --accept`. A run's
  line and `--json` say when the machine held less than the whole declaration.
- A body starts once the decision is made, in its own directory, with a private `TMPDIR` removed after the run.
  The runtime is recorded by its real path. A word's value or a plain setting that begins with `~/` reaches a
  body as a path under your home.
- The collection declares what each reflex touches; `note`'s file is a path, under your home or absolute.
- In the SDK, a body runs held to its declaration as far as the package can, under Node's permission model on
  both systems and Seatbelt on macOS, and a result carries `contained`.
- A second adapter, `openjev`, reaches Jev through [OpenJEV](https://openjev.sh): `adapter = "openjev"` in
  `evoke.toml`, the key in `OPENJEV_API_KEY`, `[adapters.openjev]` for its gate, and `openjev()` from
  `@evoke-build/evoke/openjev` in the SDK.
- Both adapters wait out a rate limit that says how long to wait, within the decision's 30 seconds, so a batch
  such as `evoke test` completes.
- evoke.build was rebuilt around five pages: a home page with a stepped walkthrough, how it works, six examples on
  pages of their own, a page for developers, and a get-started page that switches between the terminal and the
  SDK. The old use-cases address leads to the examples.

## [0.4.0] - 2026-09-23

- A sentence of several steps is read more carefully: `don’t` with a curly apostrophe negates; a value the
  sentence already used is one task, not two; `them` over two steps confirms the plan; an empty step skips
  clean; `that the` and `this is` are no longer references; a bound value replaces the reference that named it;
  a sentence with more than two dozen connectives is decided as one input; a plan handed to the SDK's run is
  checked first.
- After a sentence of several steps, `evoke why` shows every step, and `evoke teach <call>` without an utterance
  takes the step that decided its reflex, or names the steps. A step refused with a bound value in its words says
  so; a plan stopped before any step ran logs every step and prints it under `--json`; a question out of range is
  asked again; a sentence that is only what not to do says `nothing to do`.
- `[t]each` at a confirm records the lesson and runs; an answer other than `y`, `n` or `t` is named before the
  question is asked again; what you typed at a prompt is echoed plain.
- An argument answered at a prompt is settled: the decision no longer confirms a second time over it.
- `evoke show <name>` lists the lines of your overlay that address nothing the reflex has.
- `evoke check` and `update` treat an argument made required or a config key made secret as `major`, and an
  argument made optional or a secret made plain as `minor`.
- An optional argument over an empty vocabulary no longer makes its reflex inactive; a required one still does.
- `--tag` with a tag no reflex carries says so.
- A value typed at a prompt or written in a call must read whole; a leading minus is part of a number; the digits
  after a thousands comma or a decimal point are no candidate of their own; `-5 minutes` is a number.
- A call or a lesson that names one argument twice, through its former name, is refused.
- A step that names two earlier results has both filled in.
- Under `--json`, a request that is only what not to do prints one abstain line; a failure at a prompt is the
  line's own `error`; `run --json` prints one object when the body fails, and the decision line when a confirm is
  declined. A flag where a name should be is refused as a flag; `run` says its flag goes before the call; `teach`
  refuses a flag-shaped or empty utterance.
- `evoke update` skips a reflex whose new manifest does not read and moves the rest; `evoke show` lists the
  project before `sync`; a git call ends within a minute; ssh's own words are reported; a `trust.toml` that does
  not read names itself; a runtime gone since it was recorded ends in `evoke sync`; an owned file that is a
  symlink is written through; `evoke test` with nothing to test says so; every refused `add`, `remove` and
  `update` form ends in the command that helps; a failed keep leaves nothing in the store.
- `Ctrl-C` while the spinner turns clears its line; a bug prints one line, with where and the address to report
  it; `evoke check` on a body that does not load says why; a thrown error's frames start at its first frame.
- A body's result of any size reaches `evoke` and the SDK whole; a line that does not read is echoed to its first
  120 characters.
- A file body starts sooner, and in a sentence of several steps the next step's runtime starts while the current
  one runs.
- An engine answer that fails validation is asked again on the next run; probabilities that do not sum to 1 are
  refused as malformed, exit 4, in both hosts; a recorded yes/no answer that carries `no` must sum to 1.
- A key the classifier refuses ends in `export TYPESAFE_API_KEY=<value>`, exit 4; an empty key counts as unset.
- Both hosts read the proxy from `HTTPS_PROXY` and `NO_PROXY`, the lower-case names first; an address that is not
  `http` or `https` is refused; a proxy's refusal names the proxy; the CLI follows no redirect from the endpoint.
- Over the classifier's option limit on a vocabulary argument, the line names the vocabulary and the word to
  remove.
- A control character in a local reflex path, a plain setting or a git URL is refused; a password or a token in a
  git URL is refused; `.` and `..` are no ref segments; a path typed as a ref is told how a local reflex is
  written.
- A digest in a lock or a trust file is lower-case hex only.
- `teach` reads an utterance in NFC, as a decision does.
- Lint's "addresses the model" matches whole words.
- A manifest saved with Windows line endings says so.
- The SDK: a lone surrogate, a name that is no name, an adapter answering `NaN` and a file that will not read are
  each the diagnostic, fault or failure they are; `handle` declines on the second answer that does not read; an
  empty input is refused before the adapter is asked; two test files recording into one `answers.toml` keep each
  other's answers; a body's own error is a `FailureError`'s `cause`; the weave's `At` is now `Turn`.
- The SDK needs Node 24.5 or newer. Through a proxy, a connection that never answers ends at the deadline.
- The collection: `mail` is `write`; `trash` says `destructive`; `wifi` finds the Wi-Fi device by its port's name
  and is a file body; `note` starts on a fresh line; `lock`, `mail`, `power`, `visit` and `volume` report a failed
  command's own words; `screenshot` tells the deadline's end from a cancel; `visit` names the word to give a URL.
  The collection needs Node 24 or newer.
- The project schema reads `ssh://git@host/…` and reserves `weave`; the schemas point at the manual.
- evoke.build: a FAQ; a privacy policy; the trademark rule stated in `TRADEMARK.md`, the README, the manual's
  index and the FAQ; `/llms.txt` and `/llms-full.txt`, and every page of the manual served beside its markdown.

## [0.3.0] - 2026-09-23

- A sentence that asks for several things is read as steps, each decided as one input is, run in the order the
  words give, a result of one step threaded into a later one where its manifest declares `[yields]`; the plan is
  shown before anything runs, and the exit code is the worst step's. `try` shows the plan; `--json` prints one
  line per step. The SDK's `steps` returns the plan and `weave` runs it. [Weaving](docs/manual/use/weaving.md).
- A manifest may declare `[yields]`: per field of the body's `data`, the kind that reads it. It is contract: a
  yield added is minor, removed or changed major; an overlay cannot set it; `show` prints it.
- A question id may be `weave.<name>`; `weave` joins the reserved names.
- Under `--json`, a body's failure is the line's own `error` and nothing more prints.
- The answer cache is keyed by the questions asked as well as the sentence.
- `evoke run --json <call>` prints the call and its result as one JSON line.
- The SDK connects through the proxy `HTTPS_PROXY` names; `NO_PROXY` excludes hosts.
- A connection that fails is said plainly, the proxy named when it carries the connection.
- `SECURITY.md` says where a vulnerability is reported. The manual's security page says how each claim is tried
  without a key.
- evoke.build: *The idea*; *The reflex format*, with the four schemas at their `$id`; a weave on the landing
  page; its fonts shown from the first load.

## [0.2.0] - 2026-09-22

- `evoke kill the lights` needs no quotes. Every argument error ends in `evoke --help`.
- `evoke help`, `evoke add --help` and `-V` are what they say; a command word after a flag is refused with the
  way to write it; the help names the manual and fits the terminal's width.
- `show`, `vocab`, `why`, `try`, `test` and `check` print their answer on stdout; what `evoke` says about a run
  stays on stderr; an answer is coloured where stdout is a terminal.
- `evoke add ./dir` installs a local reflex. A git URL may name its user, `ssh://git@github.com/owner/repo`. A
  tag is fetched once, however many reflexes come from it, with a spinner.
- The theft test at `add` decides a few cases at a time and reports what stopped it; the add stands either way.
- `evoke teach dim lights state=dim` teaches the one word `dim`.
- `evoke update` and `evoke sync` say `up to date` when nothing moved; `update`, `sync` and `test` with nothing
  installed say so.
- A decision no longer lists the inactive reflexes first; an abstain names them once.
- `try` and `why` set the top answer in bold, list the best `fits` first and fold the near-zero answers into a
  count.
- The first line without a key says where one comes from.
- A failure names a path under your home as `~/…`; a missing `git` or `node` is named as such; a private
  repository is reported as not found or private; the line to run again never repeats an input over the cap.

## [0.1.0] - 2026-09-21

The first release: the CLI, the package manager and the SDK.

- `evoke "<sentence>"` decides among the installed reflexes and runs, confirms, asks or abstains; a REPL, a pipe
  and `--json` for scripts.
- `add`, `remove`, `update`, `sync` and `trust`: reflexes fetched from git at a tag, pinned by content in
  `evoke.lock`.
- `teach`, overlays, vocabularies and `config`: wording tuned in files you own, never touched by an update.
- `new`, `check` and `test`: a reflex written, typed and tested in ten minutes.
- `@evoke-build/evoke`: the same decisions inside a Node application, over the same core as WebAssembly.
- `evoke-build/reflexes`: thirteen reflexes for what a Mac does at a word.
