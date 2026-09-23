# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- `[t]each` at a confirm records the lesson and runs, as the manual says. Before, an utterance the core would
  not file, a pasted bidi mark for instance, stopped the run the person had just said yes to; now the refusal
  prints and the run goes ahead. A confirm answered with anything but `y`, `n` or `t` says so before it asks
  again, and what you typed at a prompt is echoed plain, never raw.
- Under `--json`, a failure at a prompt is the line's own `error`, one object per input, and a line written at a
  prompt shows on the terminal. `evoke why`'s own failures end in `evoke why`. A flag where a name should be,
  `evoke show --json`, is refused as a flag; `run` says its flag goes before the call; `teach` refuses a
  flag-shaped utterance and an empty one.
- `Ctrl-C` while the spinner turns clears its line before `evoke` ends. The one line `evoke` prints when it hits
  a bug is one line, with where, ending in the address to report it.
- `evoke update` no longer stops at a reflex whose new manifest does not read: it is reported at its new tree in
  the store and skipped, and the rest move. `evoke show` lists the project before `sync`, each missing tree as an
  inactive line, where it refused. A git call that has not finished within a minute is a failure, never a hang. What
  ssh said is what the failure says, not git's "and the repository exists". A `trust.toml` that does not read
  names itself, and `evoke trust` writes it afresh. A runtime recorded on this machine and gone since ends in
  `evoke sync`. An owned file that is a symlink is written through, and a symlinked local reflex keeps the name
  you typed. `evoke test` with nothing to test says so. A collection refused by one taken name ends in the line
  that adds the rest; a ref with a tag it has not, a collection with `--as`, a word to remove that is not there
  and a directory that is not there each end in the command that helps. `update <name> --accept <other>` is
  refused instead of dropping the accept. A failed keep leaves no `.tmp` directory in the store.
- `evoke show <name>` lists the lines of your overlay that address nothing the reflex has, as `update` reported
  them once.
- `evoke check` on a body that does not load says why without an empty line first.
- A step that names two earlier results has both written over their references; before, the second replaced
  the first. A `-5 minutes` is a number, never a five-minute duration.
- Under `--json`, a request that is only what not to do prints one abstain line instead of nothing. After a step
  that ran in rounds, `why` and `teach` see the whole weave again.
- `--tag` with a tag no reflex carries says so and ends in `evoke show`. Before, it reported the first inactive
  reflex, whatever that reflex's tags. A tag only an inactive reflex carries still explains that reflex.
- An optional argument over an empty vocabulary no longer makes its reflex inactive: the argument is never asked
  and never stated, and the reflex runs without it. The collection's `download` works the moment it is installed,
  and takes a place once `places` has words. A required argument's empty vocabulary still makes the reflex
  inactive, with the line that fills it.
- The collection: `mail` is `write`, since a draft is written; `trash` says `destructive` in its manifest rather
  than by default; `wifi` finds the Wi-Fi device by its port's name instead of assuming `en0`, a file body now, so
  the next tag carries it; `note` starts on a fresh line when the file's last line has none; `lock`, `mail`,
  `power`, `visit` and `volume` report a failed command's own words instead of Node's `Command failed` line;
  `screenshot` tells the deadline's end from a cancel; `visit`'s fix line names the word to give a URL. Eleven of
  the thirteen need Node 24 or newer, and the pages say so.
- The project schema reads `ssh://git@host/…`, the form the manual documents and `evoke add` accepts, and
  reserves `weave` as the binary does; the manifest schema wants the summary line the reader wants; the schemas
  point at the manual on the site; a `[adapters.replay]` table takes what a later `replay` may need. The
  first-reflex page's `try` block is what the binary prints.
- The SDK's doors hold. A lone surrogate in an input, a name handed to `load` or `with` that is no name, an
  adapter answering `NaN`, a file that will not read where the CLI would skip or refuse it: each was reported as
  a bug in `evoke`; each is now the diagnostic, fault or failure it is, with its fix. `handle` declines on the
  second answer that does not read, as its page said, where it threw. An empty input is refused before the
  adapter is asked. Two test files recording into one `answers.toml` no longer lose each other's answers: the
  file is read again and merged before each write. A body's own error is a `FailureError`'s `cause`. The
  weave's `At` is `Turn`, since `At` already names a line of a file. The first hour is written with `10 minutes`,
  which the duration recognizer reads, where `ten minutes` could never have run. Ten more op families are pinned
  by vectors, and a family the runners do not list fails their check.
- A file body starts sooner: the loader warms Node's type stripper while the decision is made, a start-up cost a
  body paid after it, and in a sentence of several steps the next step's loader is started while the current one
  runs.
- After a sentence of several steps, `evoke why` shows every step under its number with what became of it, and
  `evoke teach <call>` with no utterance takes the step the lesson's reflex decided, or names the steps. A step
  decided again with a bound value in its words and refused then says so, `"…" · no reflex`, where before it ended
  the run in silence. A plan stopped before any step ran, its question or prompt declined or with no terminal to
  ask, logs every step's line and prints it under `--json`. A question asked up front is asked again while the
  answer is out of range, as one input's is. One step beside a part you said not to do is decided as one input;
  a sentence that is only such parts says `nothing to do` instead of exiting in silence, and `try` says it too.
- A sentence of several steps is read more carefully. `don’t` with the apostrophe a Mac types is a negation like
  `don't`. A second value the whole sentence already saw, "set a timer for 10 minutes and 30 seconds", is one
  task that confirms with its unused span, not two timers. `them` over two steps that both yield what the next
  takes confirms the plan when only one binds; before, the second was dropped in silence. A step that found
  nothing to do, an empty list, skips clean: it stops nothing after it, and the run's status is `ran`, not
  `skipped`. `that the` and `this is` are no longer read as references, `the address` beside a step that found
  an address is that one thing, and a value bound from one step is written over the reference that named it. A
  sentence with more than two dozen connectives is decided as one input instead of asking a question per
  connective. A plan handed to the SDK's run is checked before it runs.
- `evoke check` and `update` see two more contract changes: an argument made required or a config key made secret
  is `major`, since a call, an example or a plain setting stops working; an argument made optional or a secret
  made plain is `minor`. Before, all four read as wording, and a "patch" could break a call or turn a reflex
  inactive.
- An argument you answer for at a prompt is settled: its judgment leaves the gate, so a decision that asked for
  the room no longer confirms a second time over the room you just typed. The other judgments still gate it.
- A value typed at a prompt or written in a call by name must read whole. Before, `volume level=1e3` ran with
  `1`, `-5` ran with `5` and `1 hour 30 minutes` started a one-hour timer. Now a value that does not read whole
  is refused, or asked again. A leading minus is part of a number, in a sentence too, so `-5` reads as `-5`.
- The digits after a thousands comma or a decimal point are no candidate of their own: `1,000 seconds` no longer
  proposes a zero-second `000 seconds`, and `0.4 seconds` no longer rounds to a timer of nothing.
- A call or a lesson that names one argument twice through its former name, `state=off power=on`, is refused.
  Before, the last value won without a word.
- A recorded or coded adapter's yes/no answer that carries `no` must sum to 1 with `yes`, as a choice must.
- A digest in a lock or a trust file is lower-case hex only; `+f` pairs no longer pass as `0f`.
- `[t]each` and `evoke teach` read an utterance in NFC, as a decision does, so a span the decision found is found
  in the utterance it came from.
- Lint's "addresses the model" matches whole words: "as an aid" and "the models" are no longer named.
- A manifest saved with Windows line endings says so: "contains a carriage return (U+000D); save the file with LF
  line endings".
- A control character in a local reflex path, a plain setting or a git URL is refused, as it is in every other
  string of a project. A password or a token in a git URL is refused: git's credential helper holds it, never
  `evoke.toml` or the lock. `.` and `..` are no ref segments, and a path typed as a ref is told how a local
  reflex is written.
- Over the classifier's option limit on a vocabulary argument, the line names the vocabulary and ends in
  `evoke vocab <name> remove <word>`, not in removing the reflex.
- Both hosts read the proxy from the same two variables, `HTTPS_PROXY` and `NO_PROXY`, the lower-case names
  first. Before, the CLI also read `ALL_PROXY` and `HTTP_PROXY`, so the two hosts could use different proxies. A
  value that is not an `http` or `https` address is refused with the `export` line, never bypassed in silence. A
  proxy's refusal names the proxy, an offline machine reads "could not resolve", and the CLI follows no redirect
  from the endpoint.
- The SDK needs Node 24.5 or newer: the proxy support it relies on arrived there, and an older Node connected
  around the proxy without a word. Through a proxy, a connection that never answers ends at the deadline.
- A key the classifier refuses ends in `export TYPESAFE_API_KEY=<value>`, exit 4, in both hosts. Before, the line
  said to run the command again. An empty `TYPESAFE_API_KEY` counts as unset, and the SDK's missing-key line reads
  as the CLI's.
- An engine answer to a question of the weave that fails validation is asked again on the next run. Before, it was
  kept in the answer cache and served every time, exit 4 each time.
- A JavaScript body's result of any size reaches `evoke` and the SDK whole. Before, a result over 64 KiB, text or
  data, failed as "the result line is not JSON" and echoed 64 KiB of itself. A line that does not read is echoed
  to its first 120 characters.
- `evoke run --json` prints one object when the body fails, and prints the decision line when a confirm is
  declined, as a decided input does.
- The frames a thrown error prints start at its first frame. A message of several lines is no longer repeated
  above them.
- An engine answer whose probabilities do not sum to 1 is refused as malformed, exit 4, in both hosts. Before, the
  Jev adapter scaled any answer to 1, so a deflated answer read as certainty. Two-decimal rounding is still allowed
  for.
- evoke.build answers the questions people ask, on a page of its own, *Frequently asked questions*: what a reflex
  is, what the number means, what it needs, where reflexes come from and who answers for what they do, what leaves
  your machine, and who makes it — twenty-five short answers, each pointing at the page of the manual that holds
  the long one, and served to search engines as the page's own structured data.
- evoke.build states its privacy policy: no cookie, no analytics, no account, nothing held by us; what one request
  carries from your machine to the classifier and what never travels; what stays on your machine and how to forget
  it; a reflex as a program that runs as you; the SDK under its application's policy; the four parties that see
  something, each under its own terms; and the page's date, which is the date of its last commit. The FAQ and the
  policy are linked from every footer, and from a line under the card that closes every page of the manual.
- The trademark rule is stated as a rights clause, in `TRADEMARK.md`, the README, the manual's index and the FAQ:
  except for displaying the licence details and identifying us as the origin of the software, you have no right
  under the licence to use our trademarks, trade names, service marks or product names. The footers of evoke.build
  name the licence and the engine, nothing more.
- evoke.build reads well to a language model. `/llms.txt` is the index: what `evoke` is and what holds in any
  description of it, written once, then the site's pages and every page of the manual, taken from the manual's own
  index and linked as the markdown each was made from. Every page of the manual is served beside its markdown,
  `install.html` beside `install.md`, and its head names the twin. `/llms-full.txt` is the whole manual in one file,
  in reading order, each page under its address, every link absolute. What the two files tell a model to fetch, the
  build checks it serves.

## [0.3.0] - 2026-09-23

- evoke.build explains the idea. A page of its own, *The idea*: why reflexes, in ten minutes — the problem trust
  poses, the move, the number, real sessions where nothing ran on a guess, the recipe, your words, a weave, the
  road ahead and the execution it rests on — and it leads every bar. The landing page says one core, three ways
  in, and what a calibrated number buys; its title and lede carry the gate; its security section shows a
  destructive sentence confirmed at 0.97 and still asked; the classifier's section reads *Jev answers. The gate
  decides.* The README follows the same arc. The manual reads in the landing page's palette, its prose a step
  below the ink.
- evoke.build has the reflex format as a specification page, *The reflex format*: `reflex = 1`, frozen since
  0.1.0, section by section, with the four schemas served at the `$id` each declares.
- evoke.build shows in its own faces from the first load. The fonts were declared `optional`, which drew a first
  visit in the fallback face and left it there; they now wait the moment a preloaded font takes, on every page
  and in the manual.
- evoke.build shows a weave. The landing page reads «look up dana's address and email them» into its two
  steps, plans and runs it on a loop, the address threaded from the lookup into the mail, beside a confirm
  declined at its turn and a part that matches nothing; the use-cases page carries the same session as files
  and in the SDK's tests, and the manual's closing card names it. Every line is the weave transcript's.
- A sentence that asks for several things is read as steps — «kill the lights in the den and start a 10 minute
  timer» — each decided as one input is, run in the order the words give, a result of one step threaded into a
  later one where its manifest declares `[yields]`; the plan is shown before anything runs, a step's own
  questions asked first, and the exit code is the worst step's. `try` shows the plan and each step's judgments;
  `--json` prints one line per step. The SDK's `steps` returns the plan and `weave` runs it under `handle`'s
  handlers, each told which step asks. [Weaving](docs/manual/use/weaving.md).
- Under `--json`, a body's failure is the line's own `error` and nothing more prints: one object per input, as
  the format says. It used to be followed by the diagnostic as a second object.
- The answer cache is keyed by the questions asked as well as the sentence, so a decision narrowed with `--tag`
  keeps its entry beside the full one instead of overwriting it.
- `evoke run --json <call>` prints the call and its result as one JSON line, with nothing judged; with no
  terminal, the line stands for a destructive call's confirm, as it does for a decision.
- A question id may be `weave.<name>`: a question `evoke` asks on its own account, which an adapter answers
  like any other and a recording holds. `weave` joins the reserved local names.
- A manifest may declare `[yields]`: per field of the body's `data`, the kind that reads it, or a list of records
  with such fields. It is contract: `evoke check` reports a yield added as minor, removed or changed as major,
  and an overlay cannot set it. `show` prints it.
- The SDK's transport connects through the proxy `HTTPS_PROXY` names, as the CLI's does; `NO_PROXY` excludes
  hosts. The manual's environment page lists both.
- A connection that fails is said plainly: `could not connect to api.typesafe.ai: connection refused`, the proxy
  named when it carries the connection, in place of the library's line with an OS error number.
- `SECURITY.md` says where a vulnerability is reported: GitHub's private form, never a public issue. The manual's
  security page says how each claim is tried without a key, why the endpoint is built in, and that a local
  reflex is outside trust.

## [0.2.0] - 2026-09-22

- `evoke kill the lights` needs no quotes: the words after `evoke` are one input, as the manual said. Every
  argument error ends in `evoke --help`.
- `evoke help`, `evoke add --help` and `-V` are what they say. A command word written after a flag, like
  `evoke --json try …`, is refused with the way to write it, never decided. The help names the manual, explains
  a call, and lays its columns by the terminal's width.
- `show`, `vocab`, `why`, `try`, `test` and `check` print their answer on stdout, so `evoke show | grep timer`
  finds the row; what `evoke` says about a run stays on stderr. An answer is coloured where stdout is a terminal.
- `evoke add ./dir` installs a local reflex, written to `evoke.toml` relative to that file. A git URL may name
  its user: `ssh://git@github.com/owner/repo` is a ref. `sync`, `update` and `add` list a repository's tags and
  fetch a tag once, however many reflexes come from it, with a spinner while they do.
- The theft test at `add` decides a few cases at a time, counts them in the spinner, and reports what stopped
  it — the classifier, or a key not yet set — with `evoke test`; the add stands either way. `test` decides and
  counts the same way.
- `evoke teach dim lights state=dim` teaches the one word `dim`: a first word followed by a call is the
  utterance, unless it names an installed reflex.
- `evoke update` and `evoke sync` say `up to date` when nothing moved; `update`, `sync` and `test` with nothing
  installed say so.
- A decision no longer lists the inactive reflexes first. An abstain names them once, with `evoke show`.
- `try` and `why` set the top answer of every judgment in bold, list the best `fits` first, and fold the
  answers that would print as `0.00` into a count. `show` prints a pinned ref once, not with its tag beside it.
- The first line without a key says where one comes from: `jev needs TYPESAFE_API_KEY, a key from typesafe.ai`.
- A failure names a path under your home as `~/…`, without the OS error number; a missing `git` or `node` is
  named as such; a repository GitHub hides behind a credential prompt is reported as not found or private; a
  thrown error's frames print without repeating its message; an answer a prompt refuses is quoted; the line to
  run again never wraps a line already made, and never repeats an input over the cap.

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
