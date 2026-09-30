# The first ten minutes

Ten minutes, from an empty machine to a reflex that learned a phrase of yours. Every output line below is what
the terminal shows.

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

`evoke-build/reflexes` is the first-party collection: fourteen reflexes for what a Mac does at a word. Installing
a repository installs every reflex in it.

```text
$ evoke add evoke-build/reflexes
+ awake       evoke-build/reflexes/awake 0.1.0       write        runs awake.mts
              needs runs caffeinate
+ download    evoke-build/reflexes/download 0.1.0    write        runs download.mts
              needs writes {to} ~/Downloads · hosts *
+ lock        evoke-build/reflexes/lock 0.1.0        write        runs lock.mts
              needs runs open
+ mail        evoke-build/reflexes/mail 0.1.0        write        runs mail.mts
              needs runs open
+ note        evoke-build/reflexes/note 0.1.0        write        runs note.mts
              needs writes {file}
+ open        evoke-build/reflexes/open 0.1.0        read         runs open
              needs reads {place}
+ power       evoke-build/reflexes/power 0.1.0       destructive  runs power.mts
              needs runs osascript
+ screenshot  evoke-build/reflexes/screenshot 0.1.0  write        runs screenshot.mts
              needs writes ~/Desktop · runs screencapture
+ sleep       evoke-build/reflexes/sleep 0.1.0       write        runs sleep.mts
              needs runs pmset
+ timer       evoke-build/reflexes/timer 0.1.0       write        runs timer.mts
              needs runs osascript
+ trash       evoke-build/reflexes/trash 0.1.0       destructive  runs osascript
+ visit       evoke-build/reflexes/visit 0.1.0       read         runs visit.mts
              needs reads ~/Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist · runs open plutil
+ volume      evoke-build/reflexes/volume 0.1.0      write        runs volume.mts
              needs runs osascript
+ wifi        evoke-build/reflexes/wifi 0.1.0        write        runs wifi.mts
              needs runs networksetup
  inactive  note: config "file" is not set      →  evoke config note file <value>
  inactive  open: vocabulary "places" is empty  →  evoke vocab places add <word> "<meaning>"
  inactive  visit: vocabulary "sites" is empty  →  evoke vocab sites add <word> "<meaning>"
```

One row per reflex: its local name, where it came from, its tag, its effect, and what it runs. Under it, what
it touches: the paths, the network as `hosts *`, the programs; a row without that line declares nothing, and its
body touches nothing but its own folder. The kernel holds each body to that line: [Security](../security.md).
Then the **inactive** lines. A reflex that needs your words or a setting stays out of every decision until it has them. Each
line ends with the command that gives it what it lacks. Nothing is guessed for you.

## 3. Use it

```text
$ evoke "set the volume to 40 percent"
  volume level="40 percent"  0.96
volume 40%
```

The indented line is `evoke`'s own, on stderr. It shows the **call** it chose and its **confidence**. The last
line is the reflex's result, on stdout. The confidence was over the bar for a `write` reflex, so it ran without
asking.

A less certain decision confirms first. `[t]each` records what you meant, then runs:

```text
$ evoke "kill the wifi"
  wifi state="off" · write · weakest: state 0.86
    a write runs at 0.90 or more
  Turn Wi-Fi off?  [y]es [n]o [t]each > t
+ overlays/wifi.toml  [examples] "kill the wifi" = { state = "off" }
wi-fi off
```

That one line in `overlays/wifi.toml` is yours. It is an example the classifier now sees, in a file no update
will touch.

A destructive one always confirms, however sure:

```text
$ evoke "restart the computer"
  power action="restart" · destructive · weakest: route 0.98
    it cannot be undone, so it always waits for a yes
  Make the laptop restart now?  [y]es [n]o [t]each > n
[2]
```

`[2]` means *declined*: you said no, or nothing fit. When what you said fits nothing, `evoke` abstains and shows
you the ranking, and reminds you of what is not yet in play:

```text
$ evoke "what time is it"
  none of them 0.85 · timer 0.10 · awake 0.03 · lock 0.02
  note, open and visit are inactive  →  evoke show
[2]
```

## 4. Give it your words

Some reflexes need words only you can supply: your folders, your sites. A **vocabulary** is that list. It is one
file you own, and every reflex that names it reads it.

```text
$ evoke vocab places add desktop "The desktop." --value "~/Desktop"
+ vocab/places.toml  desktop = { what = "The desktop.", value = "~/Desktop" }
$ evoke "open my desktop folder"
  open place="desktop"  0.95
```

The meaning is what the classifier reads. The value is what the reflex receives: `open` opened that folder. A
setting works the same way:

```text
$ evoke config note file "~/notes.txt"
+ evoke.toml  [config.note] file = "~/notes.txt"
```

The notes file must exist, since the reflex declares it: `touch ~/notes.txt` first. A secret is only ever named,
never stored: `evoke config <reflex> <key> --env <VAR>` writes the variable's name, and the value is read from
your environment when the reflex runs.

## 5. Teach it

Teaching works from the command line too, and you can teach what something is *not*:

```text
$ evoke teach "go dark" wifi state=off
+ overlays/wifi.toml  [examples] "go dark" = { state = "off" }
$ evoke teach "what time is it" not timer
+ overlays/timer.toml  [examples] "what time is it" = false
```

## 6. Look inside

`try` decides without running and shows every judgment. `why` explains the last sentence. `show` prints what is
installed, or one reflex as it is used, with `+` next to every line that is yours.

```text
$ evoke try "kill the wifi"
  "kill the wifi"

  which reflex            wifi 0.95   (none of them 0.03 · power 0.01)
                          Turn Wi-Fi on or off.
  state                   off 0.86   (on 0.10 · not said 0.04)
                          asked a second way: off 0.86; the less sure of the two counts
                          is it off? yes 0.95
  → waits for a yes: wifi state="off" · write · weakest: state 0.86
    a write runs at 0.90 or more

  replay answered 22 questions in 2 rounds
```

Which reflex, then each value with what it stands on, then what would happen and why. The weakest answer is the
confidence. It sits under the write bar, so this one would wait for a yes.

## Where things are

| What                                   | Where                                             |
| :------------------------------------- | :------------------------------------------------ |
| Your project: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/` | `~/.config/evoke/`                      |
| Installed code, keyed by content hash  | `~/.cache/evoke/store/`                            |
| The decision log, the REPL's history   | `~/.local/state/evoke/`                            |

Your project holds only what you wrote, the lock, and generated types. Put it in your dotfiles. Everything else
is a cache, and `evoke sync` rebuilds it on a new machine.

**Next:** [Concepts](concepts.md), or straight to [Typing a request](../use/saying-things.md).
