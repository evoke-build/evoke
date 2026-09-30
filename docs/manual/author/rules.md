<!-- description: How to write a reflex, a playbook and a collection so that what people type reaches the right reflex: the rules, their reasons and three checklists. -->
# Rules for reflexes, playbooks and collections

The keys are defined on [The manifest](manifest.md), [Arguments](arguments.md) and [Playbooks](playbooks.md). This
page says how to write them so that what people type reaches the right reflex, is asked only for what it left out,
and never runs a step nobody asked for.

Everything the classifier knows about your reflex is your manifest, read word for word with every sentence a person
types. It chooses one reflex or none: the **route**. It reads each argument's value, or answers *unstated*. The
**gate** then turns those answers and the effect into an outcome: a call runs unasked only when every value it
holds clears the floor its effect sets and the call holds all that was typed, and otherwise `evoke` asks for the
value, or for a yes. When a reflex is added, the classifier also answers **fits** for every example installed:
does the new reflex do what that sentence asks? The person reads the call or the plan, and gives the yes.
Neither the classifier nor the person knows what you meant and did not write. So the careful thinking is yours,
done once, in the words of the file.

Four words come up throughout. A **lookup** is a reflex whose `effect` is `read`. A **change** is one whose `effect`
is `write` or `destructive`. A **record** is one line of `[examples]` or `[tests]`. A **yield** is one field of a
body's result that a later step may take, declared under `[yields]` with the kind that reads it; a whole result
travels instead by the name `returns` gives it and `takes` asks for.

Each rule gives its reason. Where the tool holds a rule for you, a word in italics after it says what: *(check)*
refuses the manifest, *(lint)* reports at `check` and `add` and never refuses, *(add)* reports when a reflex is
installed, *(plan)* refuses the sentence before anything runs. A rule with no such word is yours to keep. Each
section ends with a line that keeps its rules, one that breaks them, and what the terminal prints.

Everything you write has a reader:

| What you write | Read by |
| :-- | :-- |
| The reflex's name, the last segment of its directory or repository | The classifier, as one of the route's choices. The person, first on every call line |
| `description`, `not_for`, `[examples]` | The classifier, with every sentence. The person, in `evoke show` and in the file |
| An `ask` | The classifier, as the argument's question. The person, at the prompt |
| An option's key | The classifier, as the choice's key. The person, at the prompt and on the call line. The body |
| An option's meaning, a vocabulary word's meaning | The classifier, as an answer to the ask |
| A vocabulary word's value | The body, and `[needs]` where it names the argument. Never sent |
| `steps` | The classifier, each step as a typed sentence over the set it runs on. The person, as the plan's lines |
| `effect` | The gate, as the floor a call must clear before it runs unasked. The person, when a call confirms |
| `confirm` | The person, before the action |
| `[needs]`, `platforms` | The kernel, which holds the body to them. The person, at `add` |
| `[tests]` | `evoke test` and `evoke calibrate`. Never sent |
| `tags` | `--tag`, which narrows a request. Never sent |
| The result line | The person, after the run |

## 1. One reflex, one action

1. **One reflex is one action, with one effect and one confirm**, since the effect sets how sure `evoke` must be
   before every call of the reflex: a gentle action bundled with a harsh one confirms like the harsh one. The states
   of one action are the options of one argument, like `screenshot`'s `area`, so a sentence that names no state
   still reaches the reflex. Split the states into one reflex each, and that sentence reaches one of them by
   a narrow margin, or none.
2. When a plan runs a lookup and then the change it serves, make them two reflexes: `device` finds the laptop and
   `lock` locks it. A lookup runs without asking on weaker evidence than a change, and one lookup can feed several
   changes. Where people also type the change alone, give the change a word of the sentence to read, «Which
   colleague's laptop?», since a reflex that only takes a whole result is refused when nothing before it returns
   one. *(plan)*
3. Give a reflex only the arguments its body uses, since each argument is one more answer and a call's confidence
   is the weakest of them. An argument the body cannot run without is required, and so is a value the confirm must
   read back, since a confirm may name only required values. Make an argument optional only where running without
   it is what the person expects, since an optional value the classifier misses is dropped, never asked. When a
   value of one kind plays two roles, a day of departure and a day of return, give each role its own argument,
   named by the role, since one question cannot read two answers. When people state one value in two forms, a
   new time or a shift, «to 4pm» or «15 minutes later», make the form the confirm can show the one required
   argument, since no key makes one of two required; the other form then stops the call at confirm (rule 4).
4. A value from an open set is a pick, a switch is a flag, and options are for a closed set that covers every value
   people mean, since a value with no key is asked. When people type a kind of value beside the action and no
   argument reads it, the call holds less than they typed, and stops at confirm where that shows: so give a
   value an argument when the body can use it, and always when the body acts on a quantity.

```toml
# kept: three states of one action, one effect, one confirm
[args.area]
ask               = "The whole screen, a window, or a selection?"
options.screen    = "The whole screen."
options.window    = "One window, chosen by clicking it."
options.selection = "A rectangle you drag out."
optional          = true
```

A `screen`, a `window` and a `selection` reflex each read «fire off a quick screencap», cost three descriptions on
every request, and share the route between them, so the call stops at confirm or nothing runs.

## 2. When to write a playbook

[Playbooks](playbooks.md) has the keys. A **playbook** is a reflex whose body is a plan: `steps`, one sentence each,
in place of `run`. A **situation** is what a person faces, typed in one sentence: a problem, «checkout is failing in
eu-west», or a routine, «close the month».

5. **Write a playbook for a situation people type in one short sentence and your team handles with the same steps
   each time**, since a sentence that states the situation reaches the playbook, and one that asks for the steps
   reaches their reflexes instead: such a request seldom opens the playbook, and is refused whole when a step
   takes what the plan's steps return. Steps that differ each time are the person's own sentence, not a playbook.
6. Its summary, its examples and its tests state the situation, never a step's action, since a sentence that names
   a step reaches that step's reflex and the playbook is never reached. Its test is the short sentence a person
   types when the situation happens, in words no example holds.
7. Its `not_for` names the steps people type alone, in their words, «draining the primary on its own», and the
   situations beside it that it does not handle, so one step typed alone reaches its own reflex.
8. A plan holds one situation's steps in the order they happen, the lookups a change rests on first, since a person
   reads the whole plan before one yes; keep it to the steps the situation needs, since 24 is a cap and not a
   target. Every step keeps its own gate at its turn, a destructive one asking again, so a plan may be long: each
   step still asks what it must.
9. When what happens next depends on what a step found, make that check a reflex of its own that yields one field,
   and list the alternatives right after it, one per value, each under `when`. A step that runs either way has no
   `when`. A step may reach another playbook once; a playbook reached from that one refuses the plan. *(plan)* No
   step holds a condition, a loop or a computed value, since evoke judges no condition and computes no value of
   yours.
10. Its `effect` is the worst its steps reach, since `add` prints a claim its steps pass. *(add)*

```toml
# kept: the situation, no step named
description = """
Handle a service that is down or failing.
Checks its errors, deploys and logs, finds the release behind the failure, rolls it back, tells the incident channel and updates the status page; the rollback asks again before it runs."""
not_for = ["rolling back one release on its own", "a slow service that still answers"]
```

An example «check checkout's errors» on the playbook reaches `errors`, never the playbook; a step «check whether
errors dropped, then roll back» is reported: `step 4 holds "then"; one step is one action`.

## 3. Words that reach the right reflex

[The manifest](manifest.md) has the keys.

11. The summary, the first line of `description`, says the action and what it acts on, in the words people type,
    100 characters at most, since a stranger reads it first. *(lint)* The second line draws the boundary: what the
    reflex covers, what it touches, and what cannot be undone, since a reader takes the effect from those words.
12. Describe the action and never address the classifier, since one manifest serves every engine and every reader.
    *(lint)*
13. `not_for` names the near neighbours: what a person might type that sounds like your reflex and is not. Write
    each line in the manifest of the reflex that must not take the phrase, since a line guards only the reflex that
    carries it. Add a line for a phrase you have seen taken, or a neighbour's example your reflex would take, since
    a line also turns away your reflex's own sentences near it. Never a generic phrase: every line is read with
    every request, and a generic one holds nothing. *(lint)* reports more than eight lines.
14. Write at least three examples, as people type the request, since every example is read with every sentence
    and costs on every request. Take them from sentences people typed; varying the verb for its own sake buys
    nothing. Each asserts the options and flags its words state, which `evoke test` checks, and a pick as the exact
    span the sentence holds. *(check)* refuses a span the sentence does not hold; *(lint)* reports fewer than three
    examples.
15. Write the near neighbours as `false` tests, in words your examples do not use, since a test proves a boundary
    only where the classifier never saw its words. Cover each option and each pick. For every option or pick
    argument, write one record that leaves the value out and asserts the argument unstated, `{ label = false }`,
    since a required one is then asked. *(lint)* No record opens with a negation or a condition: typed, a negated
    part is left out and a conditional one refuses the request.
16. **Guard a change with a required value and an honest effect, never with wording alone**, since only an ask or a
    confirm stops a wrong route. A question about the change that states its value passes the ask and the confirm
    alike, so a collection with a change also holds the lookup that answers such questions (rule 39).

```toml
# kept: the action, then the boundary; the near neighbours as false tests in new words
description = """
Set a timer that rings after a duration.
From seconds to hours, named in quotes if you like, "like this"; it rings with a notification and a sound."""
not_for = ["keeping the laptop awake for a while", "an alarm or a reminder at a time of day", "asking what time it is"]

[examples]
"set a timer for 12 minutes" = { duration = "12 minutes" }

[tests]
"give me a two hour countdown" = { duration = "two hour", label = false }
"ring at 6:30 am"              = false
```

`not_for = ["anything unrelated"]` holds nothing and is read with every request; `"set a timer" = { duration = "12
minutes" }` is refused: `asserts duration = "12 minutes", which is not in the utterance`.

## 4. Words that read each value

[Arguments](arguments.md) has the keys.

17. **Each ask is one question, as a person would ask it**, since the classifier reads it word for word and the
    person sees it at the prompt. Never ask whether a value was given: every question already offers *unstated*.
    Over a vocabulary, name the kind of thing, «Which room of the house?», since a bare «Which room?» let a room
    the list lacks be read as its nearest word; and keep the question short, since a longer one lost a word the
    list holds.
18. An option's key is a word a person knows at a glance, since the prompt and the call line print keys alone:
    `[1] restart  [2] shutdown`. Its meaning answers the ask in one short line, since every request
    carries every meaning. *(lint)* reports a meaning that repeats the ask.
19. A value only the user knows, a folder, a service, a person, is a vocabulary, named by its published name below,
    since reflexes share a list only by naming it alike. Your records assert nothing about a vocabulary word, since
    the words are the user's; a user's **overlay**, the wording file kept over a shipped manifest, may.
20. A value people type is a pick of its kind, `number`, `duration`, `email`, `url`, `quoted`, `date`, `time`,
    `amount` or `code`. A recognizer reads the forms its grammar knows and never invents one: a code as it is typed,
    `TP1043` or `tp1043`; a quoted value between double quotes. A form it does not read is asked for. A `range`
    bounds a number or a duration, so a value outside it asks with the reason.
21. `quoted` is for text people quote. Typed without quotes, an optional quoted value is dropped and a required one
    is asked. So text people type freely as part of the sentence is never a `quoted` argument: the body reads it
    from `input`. For a typed sentence, `input` holds the person's own words; when a plan runs the step, it holds
    the step's words. A message a playbook sends is therefore written into the step. *(lint)* reports a quoted
    argument that no example shows in quotes.
22. Where people point at a value an earlier command found, `recent` on the pick names the field a lookup yields,
    so the ask lists the values this session found and the person chooses.
23. The confirm reads the call back, its action and each required value, `Roll back {release}?`, and nothing
    else, since when a call asks it is the last line before the action and must say what this call will do. Never
    `Are you sure?`. *(check)* refuses a confirm naming an optional argument, a flag or a taken result; *(lint)*
    reports one that names no required argument, or asks *Are you sure*.

```toml
# kept: one question, a pick, a range
[args.level]
ask   = "How loud, in percent?"
pick  = "number"
range = [0, 100]
```

`ask = "Did they say how loud, and in percent or not?"` is two questions, one of them already answered;
`confirm = "Set the volume to {level}, {muted}?"` is refused: `confirm names {muted}, a flag`.

## 5. How to word a step

A **step** is one sentence of a playbook's plan. A **slot**, `{service}`, is an argument written into a step.

24. **One step is one action, written as a person would type it, never as a call**, `check {service}'s errors` and
    not `errors service={service}`, since each step is decided over whatever is installed, as a typed sentence is.
    A step may hold a verb that is also a reflex's name; only the call form is refused. *(check)* refuses the call
    form; *(lint)* reports a connective.
25. Write the slot into every step whose reflex reads its value, required or optional, `list {service}'s deploys`.
    A vocabulary word the person typed also reaches a step written without its slot, `list its deploys`, since the
    plan carries a stated vocabulary word into a step that lacks it; a pick or an option is never carried, so its
    slot is always written. A step without the slot must not hold a word of that vocabulary, `search its logs`
    beside a service named `search`: the plan leaves such a step to read its own value, it reads none, and asks.
26. Put an optional slot in a bracket with the words that go only with it, `[ in {region}]`, and leave a required
    slot bare, since the bracket drops whole when the slot is unstated. *(check)*
27. Describe a channel, «post to the incident channel», since the step then reaches whichever word of the person's
    `channels` vocabulary means that. Never write an address into a step, since a value is never the playbook's
    own; make the recipient a slot the sentence fills. A required slot the sentence leaves out keeps the plan shut
    until it is answered, and in a longer request a part that takes what the plan's steps return then has no source
    and the whole request is refused. *(lint)* reports a channel or an address written into a step, and calls both
    a slot: describe the channel, and make the address a slot.
28. A step that takes a value an earlier step found refers to that step with `it`, `them` or its noun, `wipe it`,
    `roll {service} back from that release`, since a found value binds only through a reference; never `they`,
    which evoke does not read as one. Word a check `check whether`, never `check that`, since `that` before a noun
    names an earlier step. *(lint)*
29. Word each step apart from the situation, its summary and its examples, since a step near the playbook's own
    sentence reaches no reflex, or its own plan, and the plan refuses. *(plan)* *(lint)* reports a step whose words
    are all the summary's or an example's.

```toml
# kept: one action each, the slot where it goes
steps = [
  "check {service}'s errors[ in {region}]",
  "list {service}'s deploys[ in {region}]",
  "roll {service} back from that release[ in {region}]",
  "post to the incident channel",
]
```

`"errors service={service}"` is refused: `step 1 reads as a call: errors service={service}; a step is a sentence`.
`"post to #incident"` is reported: `step 4 states "#incident"; a word another team would change is a slot`, and the
fix is to describe the channel.

## 6. How results connect

30. **Declare under `[yields]` every field a later step may take, by its kind, named by the noun a person would
    type for it**, since a step takes only a declared field, and `that release` takes the field of that name from
    the step it refers to. A found value fills only a pick of the same kind, never an option or a vocabulary word. A
    lookup yields the value its change reads, in the change's kind and in its absolute form, a day as
    `2026-05-06` and a time as `09:30`, two digits each on the 24-hour clock, since a step and `recent` take a
    yielded value only in that form, and `9:30` is nothing.
31. A lookup that finds several things yields the one its change acts on, under that thing's noun, `job =
    "number"`, whenever a plan's step takes it. The list, `jobs = { each = { id = "number" } }`, may stand beside it
    under another noun, for `recent` and for `them`, which runs the next step once per item. Over a list, `that
    job` stops the whole plan before anything runs, `step 2 takes one job from step 1, which finds several — which
    one?`, since nothing is picked for you.
32. Give a whole result one noun across the collection, and one shape under it, since a reflex that takes it binds
    by the name alone and checks the shape itself. A reflex never returns a name it takes, and a destructive reflex
    takes a field its confirm can show, never a whole result. *(check)*

```toml
# kept: the field the next step takes, by kind and noun
[yields]
release = "code"
```

A `deploys` reflex with no `[yields]` gives «roll back that release» nothing to refer to, so the release is asked
before anything runs: the plan's line ends `· asks release`.

## 7. What a person reads

33. **Always write `effect`**, since left out it means destructive, and a reader sees that only where the
    description says the change cannot be undone. *(lint)* `read` observes. `write` changes what a later call can
    put back as it was, the volume, a light, a message to your own team. `destructive` ends or spends what no call
    brings back: a wiped disk, a purchase, a message to someone outside. A `destructive` call asks before it runs,
    every time; a `read` or a `write` runs without a question when the classifier is sure of every answer. A change
    that also sends a notice takes the notice's effect, judged by the widest audience the call's values can name,
    since a notice is never taken back. Claim the effect the action has, never a worse one to be safe, since a
    question before every harmless call teaches the yes a destructive one needs.
34. Name a reflex by its directory, which `add` installs it as: the word people use for its action or its object,
    noun or verb alike, short, unique in the project it joins, since the name opens every call and plan line and
    the classifier reads it too. Name a playbook by its situation.
35. The result line says what happened in one lowercase line, with the value that lets a person check it, `volume
    40%`, `locked`, never `done`. A lookup that found several things counts them first, `3 jobs queued`, then one
    line each.
36. Keep the keys in the order `evoke show` prints them, after `reflex = 1`: `description`, `not_for`, `tags`,
    `effect`, `confirm`, `steps` or `run`, `platforms`, `returns`, then the tables, `[needs]`, `[config]`, each
    `[args.<name>]` with its `ask` first and its source next, the asked arguments before the taken, `[yields]`,
    `[examples]`, `[tests]`. In TOML a plain key written below a table header belongs to that table, so the plain
    keys come first; and a stranger then reads the file as the tool shows it: what it does and will not take, what
    it changes and asks before acting, what it touches, what it will ask, what it was tested on.

```toml
# kept: the effect written, the keys in show's order
reflex = 1

description = """
Roll a service back from a release to the one before it.
Everywhere or in one region; live traffic moves to the older release, and only a new deploy brings the newer one back."""
effect  = "destructive"
confirm = "Roll back {release}?"
run     = "rollback.mts"
```

With `effect` left out the tool still treats the rollback as destructive, and a reader who sees no word of undo
takes it for a write; a `confirm` written below `[args.x]` belongs to that argument, and `check` says
`confirm is required`.

## 8. How to design a collection

A **collection** is the reflexes and playbooks you publish together: one repository, versioned by one tag.

37. **Write the collection's shape before any wording**: each reflex's name, effect, arguments with their sources,
    and what it returns and takes, since each reflex's words depend on its neighbours. The shape is the
    **contract**, what a user cannot override: the run or the steps, the arguments and their sources, the option
    keys, what a reflex yields, returns and takes. A reflex's words move freely under one tag; a playbook's steps
    are contract too.
38. When the collection acts on things the user names, a service or a printer, its topic is one vocabulary every
    reflex that acts on them names, `services` across an outage collection, since a word typed once reaches the
    steps that lack it only through that name. Things found by a pick, a meeting by its time or a job by its
    number, are no vocabulary: coin none just to have a topic. A vocabulary that exactly one installed reflex
    requires, a playbook among them, is that reflex's own while it stays the only one, and a typed word then
    reaches no other step through it.
39. Add a reflex only for an action the collection lacks, since every installed reflex is read with every request
    and two that cover one action take each other's sentences and stop each other at confirm. Where people ask
    about what a change changes, hold the lookup that answers, so «what does the status page say?» has a home and
    never runs `status`.
40. Every reflex a playbook's steps reach carries the tag the playbook carries, one tag per domain shared by the
    reflexes that serve it, since a request narrowed with `--tag` starves a step whose reflex lacks it and the plan
    refuses. *(plan)* *(add)* reports a step that reaches a reflex without the tag.
41. Install the collection beside the reflexes it will live with, and read every `steals` and `also fits` line: each
    names a sentence two reflexes both claim. Fix a theft in the `not_for` of the reflex that steals. *(add)* Two
    reflexes of one name are installed under different names, with `add --as`.
42. The README holds one table, a row per reflex with what it does, its effect, each argument's source, what it
    yields, returns and takes, and what the user sets, since a stranger chooses a collection there and it is the
    collection's shape.

```text
$ evoke add ./outage                               # kept: the playbook installed last, every step reaching
+ outage  ./outage  destructive  a plan of 7 steps
  1  check {service}'s errors[ in {region}] → errors · read
  …
```

A newcomer `restart` for a service prints `restart: steals "restart this computer" from power`, a sentence two
reflexes both claim. Under `--tag outage`, a step whose reflex lacks the tag prints `"list checkout's deploys" · no
reflex`, and the plan refuses.

## 9. What a body must do

43. **Declare in `[needs]` the tightest reach that runs**, since the kernel holds the body to it and a person
    consents to it at `add`. What differs per machine is a `[config]` setting, a secret only as the name of a
    variable, and `platforms` is named when the body runs on one system, so elsewhere the reflex is inactive and
    says why.
44. The body detaches what outlives the run, stops when `signal` aborts, fails by throwing in its own words, runs
    programs without a shell, and is one file, since the deadline of thirty seconds a decision, the kernel and a
    reviewer hold it to that. [The body](body.md) has the rest.

```toml
# kept: one program, named; the system it runs on
platforms = ["macos"]

[needs]
runs = ["lpstat"]
```

`runs = ["sh"]` reaches everything a shell can; a program this machine lacks refuses at `check`, whatever `platforms`
names: `[needs] runs names lpstat, which is not on PATH`.

## 10. How to work

45. **Work in the order that finds each fault first.**
    1. `evoke new` writes a reflex that checks clean, and `evoke new --playbook` a playbook.
    2. `evoke check`, after every edit, refuses what cannot load and reports lint, without the classifier, on a
       machine that runs the body.
    3. `evoke trust` once; then give your own project a word in each vocabulary the reflex requires, since a reflex
       over an empty vocabulary is inactive and `test` skips it.
    4. `evoke add` into that project beside the neighbours, which names each of their examples your reflex would
       take.
    5. `evoke try` one sentence per outcome you expect: a run, an ask, and one that is not yours.
    6. `evoke test`, which decides every record over everything installed.
    7. `evoke teach` a miss, and copy its line into `[examples]`, dropping the vocabulary argument from what it
       asserts.
    8. `evoke calibrate` over what the project gathers.
46. Read the contract diff before the tag, since a reworded step, a removed option or an argument made required is
    major. Rename an argument with `was`, so every user's overlay follows.

```text
$ evoke check                                      # kept: nothing refused, the lint weighed
  timer  write  runs timer.mts
  0.4.0  same
```

A manifest with a key evoke does not read prints `unknown  timer: args.label.not_for is not a key this evoke reads`
and exits 0: the key is lost, and the manifest loads as if it were absent, so read every line.

## A reflex handed as code

A reflex or a playbook handed to the SDK's `load` as code, `reflex({…})` or `playbook({…})`, keeps every rule above
except those about the body's files: its name is its key under `reflexes`, it has no `[needs]`, `[config]` or
`platforms`, and its body runs in your process and honours `signal`. `load` refuses what `evoke check` refuses in
a manifest, and the plan refuses as it would for any reflex; nothing lints it, nothing names what it steals, and a
key evoke does not read is dropped with no line printed. So run its records through a project's `evoke test` before
you ship it, and read it with the checklist in hand.

## What `evoke` checks for you

You need not remember these. The tool holds them itself.

**`evoke check` refuses**

- A body that is missing or does not load; a program `[needs]` names that is not on `PATH`; an argv element no
  argument fills; a `[needs]` entry naming no argument or setting, or naming a flag; a host other than `*`; a path
  this machine lacks.
- An argument with no source or two; a pick of no known kind; an argument with no `ask`; a `range` on anything but
  `number` and `duration`; `recent` beside anything but a pick; a reserved argument name; a retired argument name
  back in use, or one dropped from `was`.
- No `confirm`; a confirm naming an optional argument, a flag, a taken result or no argument.
- A record asserting a span the sentence does not hold, a key not offered, a taken argument or a vocabulary word
  in a shipped manifest; two records with one utterance.
- A step that reads as a call with a value; more than 24 steps; an optional slot outside a bracket or a required one
  inside; an argument no step names; a `when` at step 1 or on two fields.
- `takes` beside another key, on an argv body or a destructive reflex, or of a name the reflex returns; `returns`
  or `[yields]` on an argv body; a body's key on a playbook.

**`evoke check` reports and never refuses**

- A key it does not read, which is lost.
- A summary over 100 characters; a description over 1,000; more than 8 `not_for` lines; more than 24 options on one
  argument; more than 40 records in a table; a sentence or a step over 200 characters.
- A phrase that addresses a model.
- `effect` left out; fewer than three examples; an option or a pick that no record leaves out; a `quoted` argument
  that no example shows in quotes; a confirm that names no required argument, or asks *Are you sure*; an option's
  meaning that repeats its ask.
- A step holding a connective, stating a channel or an address, or opening with `check that`; a step whose words
  are all the summary's or an example's.

**The plan refuses before anything runs**

- A step no reflex matches; a step that takes a result no step before it returns, or two return.
- A branch on a field its source does not yield, or a value no reader reads; a `when` step that reaches a
  playbook, or that a later step takes from.
- A plan past 24 steps or three deep; a step reaching its own playbook.
- A part of the person's sentence that opens with `if`, `unless` or `in case`; a part that says what not to do,
  beside a playbook's plan.
- It stops rather than guess: a reference several fields could satisfy, or one item of a list, stops the plan
  before anything runs, naming them.
- Every request stops while one question of the project offers more than 255 choices, *unstated* counted: a
  vocabulary past 254 words, an argument past 254 options, a project past 254 reflexes.

**`add` and `test` report and go on**

- `steals` and `also fits`, each with the phrase and its owner; a claimed effect its steps pass; a step that
  reaches a reflex without the playbook's tag; a branch source that yields no such field; a step that reaches
  nothing here.
- `test` fails a record that misses, and a step that reaches nothing or its own plan.

## Exceptions

- **A change that needs nothing.** `trash` and `screenshot` need no value from the sentence, so only the gate stands
  before them. Their confirm names the action alone, `Empty the trash?`, and their `not_for` names what people type
  nearby, «logging out». (Rules 16, 23.)
- **A reflex whose only required values are taken.** `lock` takes the laptop `device` found; its confirm names the
  action alone, `Lock the laptop?`, and its examples are the words people type for it inside a longer sentence.
  (Rules 2, 23.)
- **A gentler state beside a harsher one.** States of one reflex share its effect, so the gentler one confirms as
  the harsher one does. Accept that price only where people type the action without its state; where they name it
  every time, the gentler state is a reflex of its own: `sleep`, a write, stands beside `power`, which restarts
  or shuts down. (Rule 1.)
- **A playbook fits its own steps.** `add` prints `also fits` for the sentences of the reflexes a playbook's steps
  reach, since the plan does hold that action, and no `not_for` line takes that away. Such a sentence typed alone
  still reaches its reflex. (Rules 7, 41.)
- **One verb, two things.** «restart the payments service» and «restart the laptop» share a verb across two
  collections; each names the other's object in `not_for`. (Rule 13.)
- **One exact sentence a neighbour owns.** When one whole sentence keeps reaching a reflex past its `not_for`, it
  becomes a `false` example, `"mute the sound" = false` in `volume`. (Rule 13.)
- **What every team calls the same.** A step may name a thing no team would rename, «update the status page for
  {service}». (Rule 27.)
- **A lint line is weighed.** Lint flags the first word a step would be split on, so `next` in «find the next
  flights to {city}» is flagged though the step is one action. (Rule 24.)

## Checklists

A line is answered from the files, unless it ends with what answers it: a command, the neighbours or your users. A
no sends you to the rule it names.

**Before a tag**

- `evoke check` refuses nothing, and you weighed every lint line. (45)
- `effect` is written, and it is the effect the action has. (33)
- The second line agrees with the effect. (11)
- The summary says the action and what it acts on, in the words people type. (11)
- Each argument is one the body uses, with one ask and one source. (3)
- Each optional argument has a default people mean. (3)
- Options cover every value people mean; an open value is a pick; a switch is a flag. (4)
- No `quoted` argument reads text people type without quotes. (21)
- The confirm names each required value and nothing else. (23)
- `not_for` names the near neighbours, none of them generic. (13)
- Three examples at least, as people type them, each asserting the options, flags and pick spans it states and never
  a vocabulary word. (14, 19)
- Each near neighbour is a `false` test in words no example uses. (15)
- One record leaves out each option or pick argument. (15)
- No record opens with a negation or a condition. (15)
- `[needs]` is the tightest that runs; `platforms` is named when the body runs on one system; a secret is a
  variable's name. (43)
- `evoke test` passes beside the neighbours. (45)
- `evoke add` printed no `steals` and no `also fits`. (41)
- Inside the repository, `evoke check`'s last line, `1.2.0 → 1.3.0  minor`, names the version you tag; a first
  tag has none. (46)

**Before a playbook's tag**

- The situation comes back, and your team handles it with the same steps each time. (5, your users)
- The summary and every example state the situation, and none names a step. (6)
- The test is the short sentence people type for it, in words no example holds. (6)
- `not_for` names its steps taken alone, in the words people type for them. (7)
- Each step is one action, as typed, with no condition. (24, 9)
- No step writes in a channel or an address. (27)
- No step without its slot holds a word people give that vocabulary, `search` for a service. (25, your users)
- Each slot stands in every step whose reflex reads it, an optional one in a bracket. (25, 26)
- The lookups a change rests on come first. (8)
- A later step refers to what an earlier one found with `it`, `them` or its noun. (28)
- Each check is worded `whether`, its reflex yields the field, and one step per value follows it. (9, 28)
- No step reads like the situation or like an example. (29)
- `effect` is the worst step's. (10)
- The confirm names the required slots. (23)
- `evoke add` shows every step reaching the reflex you meant. (45)
- `evoke test` fills every step from a record. (45)

**A collection's shape**

- One line per reflex: its name, its effect, each argument's source, what it returns and takes. (37)
- One reflex per action, named by the word people use, unique in the projects it joins. (1, 34)
- Each change's lookup is its own reflex where a plan needs it, and yields what the change takes. (2, 30)
- Each change people ask about has a lookup that answers. (39)
- Every vocabulary name is on the list below, or names a kind of thing none of them holds. (19)
- The things the user names are one vocabulary, or the collection has none. (38)
- The near neighbours are listed, inside the collection and toward the collections it will join. (13)
- No reflex covers an action a neighbour already covers. (39)
- Every reflex a playbook's steps reach carries the playbook's tag. (40)
- Each situation people type has a playbook, or a reason for none. (5)
- The README's table holds the shape. (42)

## Vocabulary names

A **vocabulary** is the user's list of words for one kind of thing, kept in `vocab/<name>.toml`. You name the list
in `vocab = "<name>"`; the user writes its words, their meanings and, where the body needs one, their values. A
collection never ships one. Reflexes share a list only by naming it alike, so use these names, and coin a new one
only for a kind of thing none of them holds: a plural noun for what its words are, like `vehicles` or `printers`.

| Name | Its words are | A word's value, when the body needs one |
| :-- | :-- | :-- |
| `places` | folders on this machine | the folder's path |
| `sites` | web sites | the URL |
| `rooms` | rooms of a home | an id the lights know |
| `people` | people you work with | an address or an account id |
| `teams` | teams | an id |
| `channels` | chat channels, written `#incident` | the channel's id |
| `services` | services a team runs | the service's id |
| `regions` | where a service runs, `eu-west` | an id |
| `cities` | cities | a code |
| `months` | months, `september` | none |
| `shipments` | shipments on their way | a tracking id |
| `suppliers` | suppliers | an account id |

A day, a time, an amount and a code are never a vocabulary. They are picks, read by code from what the person
typed, `tomorrow` and `3pm` among them.

A word's meaning:

- names what the word is, in the words people use for it, since a sentence or a step that describes the thing
  reaches the word through its meaning: `"#incident" = "Where an outage is handled."`;
- holds only what the word is, since every extra word draws near phrases to it: `den = "The TV room downstairs;
  also 'the snug'."` also draws «the living room», a room the list lacks;
- holds what stays true, `september = "September."`, never «last month», since the meaning is read in every
  month;
- never holds the value, since the meaning is read by the classifier and the value is not.

## Drafting with a model

A model may write a first draft of a manifest, a playbook or a whole collection. A person reads every line before
it is saved, and decides. Once saved, a manifest is words: the drafting model takes no part in any decision.

Give the model what the body does, what it touches and what goes wrong when it runs by mistake; the collection's
shape, with each neighbour's summary and `not_for`; sentences your users typed for the action, so the examples
start from real words; these rules and the checklists; what `evoke check` printed for its last draft.

Never give the model your tests, since a test proves a boundary only when its words were out of view while the
manifest was written; nor a user's vocabulary values, a setting's value or a secret.

Then read the draft as its owner. The effect, the contract and the tag are your claims, never the model's: set
`effect` yourself, since it sets the floor every call must clear. Check each example against the body: the values
it asserts are the values the body would receive. Write the tests yourself, after the draft, in words the draft
does not use. Run `check`, `add` and `test` on the draft as on any manifest, and read the lint line that names text
addressed to a model. Read it with the checklist in hand: a fluent draft is easy to approve unread.

**Next:** [The manifest](manifest.md), key by key; [Arguments](arguments.md); [Examples and tests](records.md);
[Playbooks](playbooks.md); [The body](body.md); [Publishing](publishing.md).
