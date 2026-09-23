# The collection

`evoke-build/reflexes` is the first-party collection: thirteen reflexes for what a Mac does at a word. Each
reflex is one directory, with `reflex.toml` and the file it runs, where it runs one. There is nothing to build.
Eleven of the thirteen run a file and need Node 24 or newer on your `PATH`; `open` and `trash` run a program
the Mac has.

```bash
evoke add evoke-build/reflexes
```

| Reflex       | Does                                                               | Effect      | Yours to set    |
| :----------- | :----------------------------------------------------------------- | :---------- | :-------------- |
| `awake`      | Keeps the laptop awake for a duration, or until `pkill caffeinate` | write       |                 |
| `download`   | Saves a URL's file to `~/Downloads`, or to a place you name        | write       | `places`        |
| `lock`       | Locks the screen                                                   | write       |                 |
| `mail`       | Starts an email in your mail app                                   | write       |                 |
| `note`       | Appends a dated line to your notes file                            | write       | `file`          |
| `open`       | Opens one of your folders                                          | read        | `places`        |
| `power`      | Sleeps, restarts or shuts down                                     | destructive |                 |
| `screenshot` | Captures the screen, a window or a selection                       | write       |                 |
| `timer`      | Counts down, then rings                                            | write       |                 |
| `trash`      | Empties the trash                                                  | destructive |                 |
| `visit`      | Opens one of your sites, in a private window on request            | read        | `sites`         |
| `volume`     | Sets the output volume                                             | write       |                 |
| `wifi`       | Turns Wi-Fi on or off                                              | write       |                 |

## Yours to set

A reflex that needs your words or a setting for a required argument stays inactive until it has them. `evoke`
says which line gives it what it needs. `download` runs without `places`; with it, a place you name. Two
vocabularies and one setting cover the collection:

```bash
evoke vocab places add desktop "The desktop." --value /Users/you/Desktop      # a word per folder; the value its path
evoke vocab sites add github "GitHub." --value https://github.com             # a word per site; the value its URL
evoke config note file notes.txt                                              # ~ allowed; a bare name lands under your home
```

## What they say back

One lowercase line that says what happened: `volume 40%` · `locked` · `sleeping` · `saved ~/Desktop/Screenshot 2026-09-20
at 10.31.05.png` · `copied to the clipboard` · `eggs: 3 minutes, rings at 10:34 AM` · `awake for 2 hours` ·
`noted "buy milk" in ~/notes.txt` · `new mail to ana@example.com about "friday"` · `opened https://github.com in a
private window` · `saved report.pdf to ~/Downloads (1.2 MB)`.

## How they are built

- **macOS**, one self-contained file each; a body that is not on a Mac says so and stops. `open` and `trash` are
  argv reflexes and need no runtime. `wifi` finds the Wi-Fi device by its port's name: `en0` on a laptop, often
  not on a desktop with Ethernet.
- What outlives a run detaches and returns at once. `awake` leaves `caffeinate` running. `timer` leaves a script
  that waits, then notifies with a sound.
- `power` sleeps with `pmset`, and sends the login window's own restart and shutdown events. So apps are asked
  to quit, and no second dialog appears, since the decision was confirmed already. `lock` opens the system's lock
  screen and needs no Accessibility permission.
- `download` never writes over a file already there. A download the 30 s deadline cuts short is removed, and the
  run says so. A body that runs a program reports the program's own words when it fails. `visit`
  opens a private window through the default browser's own flag, in Chrome, Brave, Vivaldi, Edge or Firefox.
  Safari opens none from a script.
- Every reflex carries at least three examples and three tests, one of them `false`. The reflexes name each other
  under `not_for`: `timer` is not for keeping the laptop awake, and `awake` is not for timers.

## Licence and changes

The collection is [MIT](https://github.com/evoke-build/reflexes/blob/main/LICENSE), since its scripts are meant
to be copied into reflexes of your own. Issues and changes go to
[evoke-build/evoke](https://github.com/evoke-build/evoke). The collection is developed and published from there.
