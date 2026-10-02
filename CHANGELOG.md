# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- A part of a sentence that matches nothing alone is read with the whole sentence: in `check the errors for
  checkout and post in #incident, payments too`, `payments too` is the errors for payments, a third step. A step
  read this way always confirms; `why` names the words and what they read as, and the JSON line carries `named`.
  A part that names a listed value its step cannot take still refuses the request.

## [0.18.0] - 2026-10-01

- More requests run without a question. Words that say what you want from the result, like `newest first`,
  no longer hold a call; `why` names them.
- A call confirms where the input may hold more than the call read: words right after a value, a text in
  quotes that no value took, a text the reflex takes that may be among the words left, words that say the call
  is wanted once more, or a value of the longer part a step was cut from. The prompt names the words; the JSON
  line carries `cut`, `again` and `quotes`.
- A value typed the way you say it is read in more forms: a day, a length of time, and a code in the shape of
  the reflex's examples, `k d oh three one five` as `KD-0315`. Where the words read more than one way, the ask
  offers what they read as; the JSON line carries them as `choices.readings`. A length written across its
  units, `1 hour 30 minutes`, reads at a prompt and in a call by name.
- A word from a vocabulary typed another way is read where your own words name it: `sept` for `september`. A
  name that may be another person's, `Samuel` beside `sam`, is asked, with the nearest word ready for a `y`;
  the JSON line carries it as `likely`.
- A sentence is cut at a sign or a letter typed for `and`: `+`, `&`, `n`. What you say you will do yourself,
  `before I forget`, and a courtesy that opens with `if`, `if you would`, are set aside and no longer read as a
  step or a condition; `why` says which, and the JSON line carries `by` on the part. After `not X,` what
  follows is kept. Where the cut at such a sign is not sure, both parts confirm, each naming the other.
- A day of the month in a step that takes from a step naming a month and a day is read beside that day: after
  `the calendar for october 12th`, `the 20th` is October 20th. `why` says beside which step; the JSON line
  carries `beside`.
- `it leaves out` names only words no value was read from; the JSON line carries `read` on words one was. A
  part that matches no reflex is never read as a text in quotes of the step beside it.
- `evoke test` names a case that passes while its call would wait for a yes; it fails nothing. The rules for
  authors say how a summary is worded for it.

## [0.17.0] - 2026-09-30

- The collection gains `sound`: mute, unmute, or turn the volume up or down by an amount.

## [0.16.0] - 2026-09-30

- `reflex.d.ts` says what `input` holds and what aborts `signal`; `evoke check` writes it again.
- The collection's manifests hold more examples of how people ask.
- A number reads in words up to the hundreds: `two hundred and forty`.
- An argument is asked when the input names a value that is not among its choices, with the reason on the
  prompt. `try` and `why` print no line for an argument the input says nothing of.
- An address, a URL, a code or a number said aloud is read as it is typed: `dana dot weiss at example dot org`.
  A day misspelt and a code typed with a space are read too, and the call confirms with the words as typed.
- A call confirms when a value from a list was read from a word of the input alone; the prompt names the word.
  `try` and `why` say under a value what it stands on, and the JSON line carries it as `basis`.
- A sentence that asks one thing is one step, whatever its connectives. Two parts that read as the same call
  are one step. A value one part states reaches another part that points at it. A playbook takes a value the
  sentence states for one of its steps. A part that asks for nothing is set aside, and the plan says so.
- A text typed without quotes is read for a `quoted` argument, and the call confirms: in `set a timer for 10
  minutes called tea` the label is `tea`.
- An ask names the words of the input that answer it: `"garage" is not on the list`. A call confirms when the
  input holds words that ask for another thing. `try` and `why` list the words no value holds, and the JSON
  line carries them as `left`.
- The bars moved: a read runs unasked from 0.80, a write from 0.90. A third bar, `whole`, holds a call that
  leaves out part of what was typed: the prompt says `holds all you said`, `try` and `why` print the line, the
  JSON line carries `whole`, and `[adapters.<name>] gate` takes the key.
- Confidence is the lowest probability among the route and the values a call holds. A flag and an argument the
  input says nothing of no longer lower it.
- A call no longer confirms for a span no argument took, for a second reflex that fits, or for a part read
  with its neighbour: `because` no longer holds `unconsumed_span`, `two_things` or `merged`. `fits` is asked
  when a reflex is added, and `try` and `why` print no `fits` line.
- A part that says what not to do and names the value meant is read with its step: `pull the payments logs,
  not in us-east, in eu-west`. A sentence that asks one thing is read whole, what it rules out with what it
  asks. An option or a flag of a call reaches a part that asks for `the same`.
- The CLI decides the parts of a sentence side by side, as the SDK does. A sentence that may be several things is
  read as one at the same time as it is asked how many it is, so the answer costs no wait of its own.
- A call that waits for a yes says why on the line under it, in the plan as at the prompt; the JSON line
  carries it as `prompt.reason`. The plan says what it folded, set aside or left out before its steps, for a
  plan of one step too.
- `try` and `why` print the sentence as it was read: how it was read as a whole, then each step with which
  reflex, each value with what it stands on and where it came from, the call with what became of it and why,
  and last how much the classifier was asked. An answer's key prints in words: `not said`, `none of them`.
- A question over a list offers `[0] none of these`, which declines it; a declined question says what did not
  run.
- `calibrate` counts the wrong calls at or over the `whole` bar too, with what the bar holds for a yes; an ask
  is no longer among the calls at a bar. The JSON carries it as `bars.whole`.

## [0.15.0] - 2026-09-29

- A code reads in small letters too: `inc-311`, `tp1043`, `c02xk1abjg5m`.
- A sentence that states a playbook's situation twice gets one plan: the second part folds into the first.
- The collection: every manifest is reworded, `sleep` is a reflex of its own, and `power` restarts or shuts
  down.
- Lint reports what a reader of a manifest would miss: `effect` left out, fewer than three examples, an option
  or a pick no record leaves out, a `quoted` argument no example shows in quotes, a confirm that reads no value
  back, an option's meaning that repeats its ask, and a playbook's step worded as its own summary. `add` says
  when a playbook's step reaches a reflex without the playbook's tag.
- `evoke new --playbook <name>` writes a playbook from the template.
- The SDK's `Manifest` carries `effect_absent` when its file left `effect` out.
- The manual: one page holds the rules for writing a reflex, a playbook and a collection, with three
  checklists, in place of the wording page.

## [0.14.0] - 2026-09-28

- Four recognizers beside the five: `pick = "date"`, `"time"`, `"amount"` and `"code"`. A date is read as the
  words say it, `tomorrow`, `next friday`, `the 14th`, `may fifth`, and reaches the body as a day of your
  machine's calendar, or of `EVOKE_TODAY`; a time as `17:30`; an amount as `{ amount, currency }`; a code, a
  version, a ticket or a serial, as typed. `[yields]` names them too, and a yielded date reaches the next step
  as a day.
- A pick may name a field with `recent = "release"`: when the words leave the argument unstated, the ask lists
  what the bodies of this session returned under that field, `From which release?  [1] 4.12.0  [2] 4.11.3`, and
  `why` names what was offered. The SDK takes the results as `recent` on `decide`, `handle` and `weave`.
- `check` prints a manifest's unknown keys, and lint flags a playbook step opening `check that`.

## [0.13.0] - 2026-09-28

- A playbook's step may run only under a value an earlier step yields: `{ say = "…", when = { landing = "yes"
  } }`, listed right after that step. The plan prints every branch, `then 4 on "yes", 5 on "no"` and `if 3
  yields landing "yes"`; the value picks at that step's turn, a step not chosen is skipped clean, `not chosen:
  step 3 yielded landing "yes"`, and a value no step lists picks nothing. `show` prints the `when`; `test` and
  `add` say when the step before a branch reaches a reflex that yields no such field.
- The SDK's `Step` carries `when`, a step skipped so says `not_chosen`, and `playbook()` takes a step as
  `{ say, when }`.

## [0.12.0] - 2026-09-28

- A reflex may be a playbook: `steps` in place of `run`, one sentence per step, `{slots}` for its arguments. A
  short sentence reaches it like any request, each step is decided over what is installed, the plan prints with
  the playbook and step that wrote each line, and one yes covers the whole; a step's own confirm is asked again
  at its turn. `add` shows what each step reaches, `test` decides the steps, `show` prints them, `check` diffs
  them as contract, and lint flags a step holding a connective or stating a channel or an address.
- A part of the sentence that repeats a step of such a plan folds into it, `folded "…" into 1`; a part that
  says what not to do beside the plan is refused, and so is a sentence that opens with `if`, `unless` or `in
  case`.
- `evoke why` shows the sentence's own block first, as step 0, when a playbook wrote the plan; `--json` prints
  that line first, without a status, and an abstain over the whole input says why, `because`.
- The SDK's `playbook()` hands one as code, `steps` takes `ask`, a compiled playbook reports `runs: "plan"`, and
  `run` and `handle` refuse one.

## [0.11.0] - 2026-09-27

- `evoke try --save <file> "<sentence>"` writes the plan as a file: the sentence, the classifier's answers and
  the pins it was decided under, in clear. `evoke run <file>` runs it exactly, here or on another machine with
  the same project, after one yes over the whole plan; a pin that moved refuses it and names the fix. Every step's
  line names the file, and `evoke why` shows it.
- The SDK's `steps` returns the plan file, and `weave` takes one.
- A plan's line says which earlier steps run beside it, `with 1, 2`, and a part that said what not to do prints
  under the plan as `left out "…"`.
- A plain setting's value no longer moves the plan digest: two machines with different values share one.

## [0.10.1] - 2026-09-27

- A word of a vocabulary one reflex alone asks for is that reflex's own: it never reaches another step.

## [0.10.0] - 2026-09-27

- A word a sentence says once for several steps reaches each of them: `check checkout's errors, deploys and
  logs in eu-west` runs every lookup on checkout in eu-west. The plan shows the word on each step, `evoke why`
  names what was shared, and `--json` carries it as `shared`.
- Two items the classifier read as one thing, `invoices, card expenses`, are two steps when each is a reflex of
  its own.
- A channel named `#it` is no longer read as `it`.
- A sentence with a change among its steps runs them in the order you wrote them. Lookups alone still run side
  by side.
- A manifest names where its body runs, `platforms = ["macos"]`. On another machine the reflex is inactive,
  and `add` and `show` say so. The collection's reflexes name macOS.
- `[+] add one` at a prompt asks for the word's path when a reflex declares its value as one it reads or
  writes, and a word without one names itself in the fix: `evoke vocab places add photos "<meaning>" --value
  <path>`.
- `evoke teach --forget "<utterance>" <name>` removes the line your overlay holds for the utterance; without
  the utterance, the last input's.
- `evoke add` refuses the same reflex under another name, and its theft test also names a newcomer that fits an
  installed example over the floor: `lamp: also fits "kill the lights" of lights (0.82)`.
- The collection's `volume` sends `mute the sound` as an example of what it is not; `visit` drops the test
  `pull up the calendar`.
- A diagnostic names its reflex once, `lamps: is not installed`. A call by name with a value out of range
  points at `evoke show`, and `evoke update --accept` of a name not installed says so.
- A folder a body writes into that is not there ends in `mkdir -p ~/Downloads`.
- `evoke config` says when a path the declaration reads or writes is not there yet.
- `evoke try` over a sentence of several steps names the inactive reflexes when a part matched nothing, as one
  input's abstain does.
- `evoke teach` and `[t]each` say when the phrase is already an example with the same values, and write
  nothing.
- A body that overruns its deadline says so in seconds. `Ctrl-C` on one body reads `cancelled` in `why` and
  on the JSON line.
- `evoke trust` in the home project says it is trusted by construction. A collection added again when every
  reflex of it is installed ends in `evoke update`.
- A local reflex outside the project is shown where it is, `~/hello`, in `show` and in a line to fix.
- A body's failure prints its message alone. The frames of an error a JavaScript body threw are kept in the
  log: `evoke why` shows them, and `--json` carries them as `frames`.

## [0.9.0] - 2026-09-26

- A manifest names what its body returns, `returns = "deploys"`, and an argument takes an earlier step's whole
  result by that name, `[args.deploys] takes = "deploys"`: filled by the plan alone, never asked, handed to the
  body beside its decision. A step takes several in one call. No step before the taker returning the name, or two
  returning it, stops the request before anything runs; so does a field taken from a step run once per record. A
  taker is refused by `evoke run` and the SDK's `run` and `handle`; a result over 1 MiB is not handed.
- `evoke show <name>` says which results a reflex takes and which installed reflexes return them.
- The collection's `download` and `screenshot` yield the path they saved to.
- `evoke try` shows a step read with its neighbour as one request as the confirm it is, in a plan of several
  steps too.
- The collection's `volume` drops the test `lower the volume by 20`.

## [0.8.0] - 2026-09-26

- A number or a duration spelled out is a candidate, as one in digits is: `fifty percent`, `twenty-five`,
  `an hour`, `half an hour`, `an hour and a half`.
- The collection's `volume` is not for turning it up or down by an amount.
- On the command line, a part read with its neighbour as one request confirms before it runs, as it does in
  the SDK.

## [0.7.0] - 2026-09-25

- `Ctrl-C` while a body runs ends the body with everything it started, then `evoke`, as an interrupted program.
  In a weave, every step that did not finish reads `skipped · cancelled`, on the terminal, under `--json` and in
  the log. The SDK's `weave` rejects with the signal's reason and puts its record on it.
- A part that is a determiner and one word, "the office", is read as another item of its neighbour's task first.
  A part read with its neighbour as one request confirms before it runs, `merged` on its line. A part put in
  place of a value must be that value.

## [0.6.0] - 2026-09-25

- `evoke calibrate` reports what the numbers meant on your own records: the whole call right by confidence,
  each bin with its count and interval, the wrong calls at or over each bar per thousand, and with `--repeat`
  the spread over repeats; the log's confirms and declines apart. `--json` prints one object; a call wrong at
  or over its bar exits 1.

## [0.5.1] - 2026-09-25

- A body that runs a program no longer prints Node's warning about `--allow-child-process`.
- Under Node 25 and later, a body that declares a host reaches the network, and Node's own layer keeps a body
  that declares none off it.
- A network reach past the declaration is named on every Node, a listening port and a datagram included. An error
  a body throws from a callback ends the run with its message.
- A failing body's frames, and `evoke check`'s, no longer include a line from inside Node.

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
