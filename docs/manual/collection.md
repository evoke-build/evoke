<!-- description: The first-party collection: fifteen reflexes for everyday Mac tasks, such as the volume, Wi-Fi, a timer or a screenshot, and what each one needs. -->
# The collection

`evoke-build/reflexes` is the first-party collection: fifteen reflexes for what a Mac does at a word. Each
reflex is one directory, with `reflex.toml` and the file it runs, where it runs one. There is nothing to build.
Thirteen of the fifteen run a file and need Node 24 or newer on your `PATH`; `open` and `trash` run a program
the Mac has. Every one follows [the rules](author/rules.md), so each is a manifest to copy from.

```bash
evoke add evoke-build/reflexes
```

| Reflex       | Does                                                               | Effect      | Reads from your words                                                    | Yields | Yours to set |
| :----------- | :----------------------------------------------------------------- | :---------- | :----------------------------------------------------------------------- | :----- | :----------- |
| `awake`      | Keeps the laptop awake for a duration, or until `pkill caffeinate` | write       | `duration`, a duration, optional                                         |        |              |
| `download`   | Saves a URL's file to `~/Downloads`, or to a place you name        | write       | `url`, a URL; `to`, one of your `places`, optional                       | `path` | `places`     |
| `lock`       | Locks the screen                                                   | write       |                                                                          |        |              |
| `mail`       | Starts an email in your mail app                                   | write       | `to`, an address; `subject`, text in quotes, optional                    |        |              |
| `note`       | Appends a dated line to your notes file                            | write       | `text`, text in quotes                                                   |        | `file`       |
| `open`       | Opens one of your folders                                          | read        | `place`, one of your `places`                                            |        | `places`     |
| `power`      | Restarts or shuts down                                             | destructive | `action`, `restart` or `shutdown`                                        |        |              |
| `screenshot` | Captures the screen, a window or a selection                       | write       | `area`, `screen`, `window` or `selection`, optional; `clipboard`, a flag | `path` |              |
| `sleep`      | Puts the laptop to sleep                                           | write       |                                                                          |        |              |
| `sound`      | Mutes, unmutes, or turns the volume up or down                     | write       | `state`, `off`, `on`, `up` or `down`; `by`, a number of points, optional |        |              |
| `timer`      | Counts down, then rings                                            | write       | `duration`, a duration; `label`, text in quotes, optional                |        |              |
| `trash`      | Empties the trash                                                  | destructive |                                                                          |        |              |
| `visit`      | Opens one of your sites, in a private window on request            | read        | `site`, one of your `sites`; `incognito`, a flag                         |        | `sites`      |
| `volume`     | Sets the output volume                                             | write       | `level`, a number from 0 to 100                                          |        |              |
| `wifi`       | Turns Wi-Fi on or off                                              | write       | `state`, `on` or `off`                                                   |        |              |

## Yours to set

A reflex that needs your words or a setting for a required argument stays inactive until it has them. `evoke`
says which line gives it what it needs. `download` runs without `places`; with it, a place you name. Two
vocabularies and one setting cover the collection:

```bash
evoke vocab places add desktop "The desktop." --value "~/Desktop"      # a word per folder; the value its path
evoke vocab sites add github "GitHub." --value https://github.com             # a word per site; the value its URL
evoke config note file "~/notes.txt"                                          # a path under your home, or absolute; a file that exists
```

## What they pass on

A reflex may declare what its result holds, for a later step of one sentence to take ([Weaving](use/weaving.md)).
`download` and `screenshot` yield `path`, the file they saved to, a quoted text: a later step with a `quoted`
argument your words left empty can take it, as `note` does in `download https://evoke.build/llms.txt and note it`.
No reflex in the collection returns a whole result under a name yet, and none takes one.

## What they say back

One lowercase line that says what happened: `volume 40%` · `muted` · `locked` · `sleeping` · `saved ~/Desktop/Screenshot 2026-09-20
at 10.31.05.png` · `copied to the clipboard` · `eggs: 3 minutes, rings at 10:34 AM` · `awake for 2 hours` ·
`noted "buy milk" in ~/notes.txt` · `new mail to ana@example.com about "friday"` · `opened https://github.com in a
private window` · `saved report.pdf to ~/Downloads (1.2 MB)`.

## How they are built

- **macOS**, one self-contained file each: every manifest names it, so on Linux each reflex is inactive, and `evoke
  add` says so. `open` and `trash` are
  argv reflexes and need no runtime. Each manifest declares what its body touches under `[needs]`, and `evoke
  add` prints it under the row: `screenshot` writes `~/Desktop` and runs `screencapture`. `wifi` finds the Wi-Fi device by its port's name: `en0` on a laptop, often
  not on a desktop with Ethernet.
- What outlives a run detaches and returns at once. `awake` leaves `caffeinate` running. `timer` leaves a script
  that waits, then notifies with a sound.
- `sleep` sleeps with `pmset`. `power` sends the login window's own restart and shutdown events. So apps are
  asked to quit, and no second dialog appears, since the decision was confirmed already. `lock` opens the
  system's lock screen and needs no Accessibility permission.
- `download` never writes over a file already there. A download the 30 s deadline cuts short is removed, and the
  run says so. A body that runs a program reports the program's own words when it fails. `visit`
  opens a private window through the default browser's own flag, in Chrome, Brave, Vivaldi, Edge or Firefox.
  Safari opens none from a script.
- Every reflex writes its `effect`, carries at least three examples, and has its near neighbours as `false`
  tests, in words no example uses. One record leaves out each option and each pick. The reflexes name each
  other under `not_for`: `timer` is not for keeping the laptop awake, and `awake` is not for timers.

## Licence and changes

The collection is [MIT](https://github.com/evoke-build/reflexes/blob/main/LICENSE), since its scripts are meant
to be copied into reflexes of your own. Issues and changes go to
[evoke-build/evoke](https://github.com/evoke-build/evoke). The collection is developed and published from there.
