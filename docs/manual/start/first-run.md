# The first ten minutes

Ten minutes, from an empty machine to a reflex that learned a phrase of yours. Every line below is what the
terminal shows.

## 1. Say something

```text
$ evoke "lock the screen"
  jev needs TYPESAFE_API_KEY, a key from typesafe.ai  →  export TYPESAFE_API_KEY=<value>
[3]
```

Two things happened. `evoke` wrote your **home project**: the file `~/.config/evoke/evoke.toml`, with one line,
`adapter = "jev"`. And it told you the one thing it needs. `[3]` is the exit code. It means *needs a human*. Export
the key and try again:

```text
$ evoke "lock the screen"
  no reflexes are installed  →  evoke add evoke-build/reflexes
[3]
```

## 2. Install the collection

`evoke-build/reflexes` is the first-party collection: thirteen reflexes for what a Mac does at a word. Installing
a repository installs every reflex in it.

```text
$ evoke add evoke-build/reflexes
+ awake       evoke-build/reflexes/awake 0.1.0       write        runs awake.mts
+ download    evoke-build/reflexes/download 0.1.0    write        runs download.mts
  …
+ wifi        evoke-build/reflexes/wifi 0.1.0        write        runs networksetup
  inactive  download: vocabulary "places" is empty  →  evoke vocab places add <word> "<meaning>"
  inactive  note: config "file" is not set          →  evoke config note file <value>
  …
```

One row per reflex: its local name, where it came from, its tag, its effect, and what it runs. Then the
**inactive** lines. A reflex that needs your words or a setting stays out of every decision until it has them. Each
line ends with the command that gives it what it lacks. Nothing is guessed for you.

## 3. Use it

```text
$ evoke "set the volume to 40 percent"
  volume level="40 percent"  0.93
volume 40%
```

The indented line is `evoke`'s own, on stderr. It shows the **call** it chose and its **confidence**. The last
line is the reflex's result, on stdout. The confidence was over the bar for a `write` reflex, so it ran without
asking.

A less certain decision confirms first:

```text
$ evoke "dim the office"
  lights room="office" state="dim" · write · weakest: state 0.70
  Set the office lights dim?  [y]es [n]o [t]each > y
group-7 lights dim
```

A destructive one always does:

```text
$ evoke "restart the computer"
  power action="restart" · destructive · weakest: action 0.97
  Really restart now?  [y]es [n]o [t]each > n
[2]
```

And when what you said fits nothing, `evoke` abstains and shows you the ranking:

```text
$ evoke "what time is it"
  none 0.70 · timer 0.20 · lights 0.05 · volume 0.05
[2]
```

## 4. Give it your words

Some reflexes need words only you can supply: your folders, your sites, your rooms. A **vocabulary** is that
list. It is one file you own, and every reflex that names it reads it.

```text
$ evoke vocab places add desktop "The desktop." --value /Users/you/Desktop
+ vocab/places.toml  desktop = { what = "The desktop.", value = "/Users/you/Desktop" }
$ evoke "open my desktop folder"
  open place="desktop"  0.91
```

The meaning is what the classifier reads. The value is what the reflex receives. A setting works the same way.
A secret is only ever named, never stored:

```text
$ evoke config note file notes.txt
+ evoke.toml  [config.note] file = "notes.txt"
$ evoke config lights token --env HUE_TOKEN
+ evoke.toml  [config.lights] token = { env = "HUE_TOKEN" }
```

## 5. Teach it

When a decision is not quite sure, `[t]each` records what you meant, then runs:

```text
$ evoke "kill the lights"
  Which room?  [1] den  [2] office  [+] add one  > 1
  lights room="den" state="off" · write · weakest: state 0.58
  Set the den lights off?  [y]es [n]o [t]each > t
+ overlays/lights.toml  [examples] "kill the lights" = { state = "off" }
den lights off
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

That one line in `overlays/lights.toml` is yours. It is an example the classifier now sees, in a file no update
will touch. You can also teach from the command line. You can even teach what something is *not*:

```text
$ evoke teach "lights out" lights state=off
+ overlays/lights.toml  [examples] "lights out" = { state = "off" }
$ evoke teach "what time is it" not timer
+ overlays/timer.toml  [examples] "what time is it" = false
```

## 6. Look inside

`try` decides without running and shows every judgment. `why` explains the last decision. `show` prints what is
installed, or one reflex as it is used, with `+` next to every line that is yours.

```text
$ evoke try "kill the lights in the den"
  lights 0.91 · none 0.06 · timer 0.02 · volume 0.01
  room   den 0.85 · unstated 0.10 · office 0.05
  state  off 0.88 · on 0.05 · dim 0.05 · unstated 0.02
  fits   lights 0.70 · timer 0.05 · volume 0.05
  run · weakest: room 0.85
```

## Where things are

| What                                   | Where                                             |
| :------------------------------------- | :------------------------------------------------ |
| Your project: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/` | `~/.config/evoke/`                      |
| Installed code, keyed by content hash  | `~/.cache/evoke/store/`                            |
| The decision log, the REPL's history   | `~/.local/state/evoke/`                            |

Your project holds only what you wrote, the lock, and generated types. Put it in your dotfiles. Everything else
is a cache, and `evoke sync` rebuilds it on a new machine.

**Next:** [Concepts](concepts.md), or straight to [Saying things](../use/saying-things.md).
