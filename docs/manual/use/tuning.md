# Tuning

A reflex ships its wording; you own the last word. Every change lands in a file under your project, written by
`evoke` with its comments and order kept, and read by every decision after. No command here needs the classifier's
key.

## `show` — what is installed, and one reflex as used

```text
$ evoke show
  lights  radhi/home/lights 1.2.0  write  runs lights.mts
  timer   radhi/timer 1.0.1        write  runs timer.mts
  inactive  lights: HUE_TOKEN is not set  →  export HUE_TOKEN=<value>
```

`evoke show <name>` prints the reflex's **effective** manifest — the shipped one with your overlay merged in — as
TOML, one key per line, with `+` in the gutter of every line that is yours:

```text
$ evoke show lights
  description = "Turn the lights in one room on, off, or dim them.\nCeiling and lamp lights only."
  …
  [args.state]
+ ask = "On, off or dim?"
  options.on = "Switch on."
  …
  [examples]
  "turn on the kitchen lights" = { state = "on" }
+ "kill the lights" = { state = "off" }
```

## `teach` — one line in your overlay

```text
$ evoke teach "kill the lights" lights state=off
+ overlays/lights.toml  [examples] "kill the lights" = { state = "off" }
$ evoke teach "what time is it" not timer
+ overlays/timer.toml  [examples] "what time is it" = false
$ evoke teach lights state=dim
+ overlays/lights.toml  [examples] "dim the office" = { state = "dim" }
```

- `evoke teach "<utterance>" <call>` — the utterance means this call. The call takes the same grammar `evoke`
  prints, `name arg=value…`, and only what you assert is recorded: `lights state=off` teaches the route and `state`,
  and leaves `room` to the input.
- `evoke teach "<utterance>" not <name>` — the utterance is never this reflex.
- The utterance omitted means the last input you gave: say something, see it decided, then `evoke teach lights
  state=dim` corrects it.
- `[t]each` at a confirm prompt does the same for the input just decided.

A value is checked at the door: a word the vocabulary lacks, an option not offered, or an argument the reflex does
not have is refused with the command that shows what is allowed.

## Overlays

`overlays/<name>.toml` is your wording for one reflex, in the manifest's own shape, merged over the shipped one by
**one rule: tables merge by key, every value replaces whole.** An overlay adds and replaces; it never deletes.

```toml
# overlays/lights.toml
description = "Turn the lights in one room on, off, or dim them."      # replaces the shipped text whole

[args.state]
ask = "On, off or dim?"                                                # new wording for an argument
options.dim = "Lower the brightness; the lights stay on."              # new wording for an existing option

[examples]
"kill the lights" = { state = "off" }                                  # sent to the classifier

[tests]
"turn on the kitchen lights" = { state = "on" }                        # a shipped example, moved here: held out now
```

What you may change is **wording**: `description`, `not_for`, `tags`, `confirm`, each argument's `ask` and the
meaning of its existing options, examples and tests. What you may not is the **contract**: what the reflex runs,
its argument names, option keys, sources and ranges. A contract key in an overlay makes the reflex inactive, with
the line to remove.

- The highest layer naming an utterance decides its table and its value: moving a shipped example under
  `[tests]` stops sending it; `= false` rejects it.
- `effect` in your overlay may **tighten** — `write` over `read`, `destructive` over either — never loosen. A
  loosening line makes the reflex inactive.
- Lists — `not_for`, `tags` — replace whole.
- Your records may assert vocabulary arguments, `{ room = "den" }`; shipped manifests may not.
- Wording in another language is just an overlay.

At `update`, an overlay follows a declared rename, and `evoke` reports what went **stale** — you override what
upstream changed, yours wins — and what is **orphaned** — yours addresses nothing now, skipped. An overlay that
fails to parse makes its reflex inactive, never silently looser.

## Vocabularies

`vocab/<name>.toml` is a closed list of your words, shared by every argument that names the vocabulary. A package
never ships or writes one; a reflex that reads an empty vocabulary stays inactive until you fill it.

```toml
# vocab/rooms.toml
den    = "The TV room downstairs; also 'the snug'."
office = { what = "The upstairs study.", value = "group-7" }
```

The **meaning** is what the classifier reads, so other names for the thing help. The **value**, when set, is what
the body receives instead of the word, and the classifier never sees it: a path, a URL, a device id.

```text
$ evoke vocab rooms
  den = "The TV room downstairs; also 'the snug'."
  office = { what = "The upstairs study.", value = "group-7" }
$ evoke vocab rooms add attic "The loft upstairs." --value group-9
+ vocab/rooms.toml  attic = { what = "The loft upstairs.", value = "group-9" }
$ evoke vocab rooms remove attic
- vocab/rooms.toml  attic
```

`add` of a word already there replaces its meaning. A word may contain spaces; `none` and `unstated` are reserved.
At an ask over a vocabulary, `+` adds a word without leaving the prompt.

## `config` — settings and secrets

A manifest declares under `[config]` what it needs from you. Each setting lands under `[config.<name>]` in
`evoke.toml`; a reflex with a setting unset stays inactive.

```text
$ evoke config lights bridge 10.0.0.2
+ evoke.toml  [config.lights] bridge = "10.0.0.2"
$ evoke config lights token hue-example
  lights: config "token" is a secret; it is set from a variable  →  evoke config lights token --env <VAR>
[3]
$ evoke config lights token --env HUE_TOKEN
+ evoke.toml  [config.lights] token = { env = "HUE_TOKEN" }
```

A **secret** is only ever set with `--env <VAR>`: your file names the variable, and the value reaches the body from
the environment at run time and nowhere else. `--env` works for any key. An undeclared key is refused with
`evoke show <name>`, which lists what the reflex declares.

## `test` — every record, judged

```text
$ evoke test
  lights  3 passed · 1 failed
    "make it darker in here"  state: expected "dim", read "off"
  timer   8 passed
  1 of 12 cases failed  →  evoke test
[1]
```

`evoke test [<name>]` decides every example and every test of every active reflex — or of one — against the whole
installed set, never through the cache, and judges each on its route and its asserted arguments. It exits 1 when a
case failed so a script can act on it; it never blocks an install.

A case that passed last time and fails now is decided twice more; failing two of three marks it `· regression`. The
last verdicts are kept per installed set, on this machine.

## Thresholds

Everything engine-specific lives under the adapter's own table, so one dotfiles repository serves machines on
different engines:

```toml
[adapters.jev]
gate = { route = 0.5, read = 0.6, write = 0.85 }
```

Each number means *the probability this is right*; `read` may not exceed `write`; destructive reflexes always
confirm. [Outcomes](outcomes.md#what-confidence-is) explains what the numbers gate.

**Next:** [Projects](projects.md).
