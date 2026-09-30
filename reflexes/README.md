# evoke-build/reflexes

The first-party reflexes for [evoke](https://evoke.build): what a Mac does at a word. The word
is decided by [Jev](https://typesafe.ai), TypeSafe AI's classifier. Each reflex is one directory, with
`reflex.toml` and the file it runs, where it runs one. There is nothing to build.
Thirteen of the fifteen run a file and need Node 24 or newer on your `PATH`; `open` and `trash` run a program
the Mac has.

```bash
evoke add evoke-build/reflexes
```

| Reflex                               | Does                                                               | Effect      | Reads from your words                                                    | Yields | Yours to set |
| :----------------------------------- | :----------------------------------------------------------------- | :---------- | :----------------------------------------------------------------------- | :----- | :----------- |
| [awake](awake/reflex.toml)           | Keeps the laptop awake for a duration, or until `pkill caffeinate` | write       | `duration`, a duration, optional                                         |        |              |
| [download](download/reflex.toml)     | Saves a URL's file to `~/Downloads`, or to a place you name        | write       | `url`, a URL; `to`, one of your `places`, optional                       | `path` | `places`     |
| [lock](lock/reflex.toml)             | Locks the screen                                                   | write       |                                                                          |        |              |
| [mail](mail/reflex.toml)             | Starts an email in your mail app                                   | write       | `to`, an address; `subject`, text in quotes, optional                    |        |              |
| [note](note/reflex.toml)             | Appends a dated line to your notes file                            | write       | `text`, text in quotes                                                   |        | `file`       |
| [open](open/reflex.toml)             | Opens one of your folders                                          | read        | `place`, one of your `places`                                            |        | `places`     |
| [power](power/reflex.toml)           | Restarts or shuts down                                             | destructive | `action`, `restart` or `shutdown`                                        |        |              |
| [screenshot](screenshot/reflex.toml) | Captures the screen, a window or a selection                       | write       | `area`, `screen`, `window` or `selection`, optional; `clipboard`, a flag | `path` |              |
| [sleep](sleep/reflex.toml)           | Puts the laptop to sleep                                           | write       |                                                                          |        |              |
| [sound](sound/reflex.toml)           | Mutes, unmutes, or turns the volume up or down                     | write       | `state`, `off`, `on`, `up` or `down`; `by`, a number of points, optional |        |              |
| [timer](timer/reflex.toml)           | Counts down, then rings                                            | write       | `duration`, a duration; `label`, text in quotes, optional                |        |              |
| [trash](trash/reflex.toml)           | Empties the trash                                                  | destructive |                                                                          |        |              |
| [visit](visit/reflex.toml)           | Opens one of your sites, in a private window on request            | read        | `site`, one of your `sites`; `incognito`, a flag                         |        | `sites`      |
| [volume](volume/reflex.toml)         | Sets the output volume                                             | write       | `level`, a number from 0 to 100                                          |        |              |
| [wifi](wifi/reflex.toml)             | Turns Wi-Fi on or off                                              | write       | `state`, `on` or `off`                                                   |        |              |

No reflex returns a whole result under a name, and none takes one.

## Yours to set

A reflex that needs your words or a setting for a required argument stays inactive until it has them. `evoke`
says which line gives it what it needs. `places` names your folders, and each word's value is its full path.
`sites` names your sites, and each value is its URL. `download` runs without `places`; with it, a place you name.
`note`'s file is a path, under your home or absolute, and it must exist: the reflex declares it and writes nothing
else.

```bash
evoke vocab places add desktop "The desktop." --value "~/Desktop"
evoke vocab sites add github "GitHub." --value https://github.com
evoke config note file "~/notes.txt"
```

## What each one touches

A manifest declares what its body touches, under `[needs]`, and evoke holds the body there. `evoke add` prints
each declaration under its row. `screenshot`'s:

```toml
[needs]
writes = ["~/Desktop"]
runs   = ["screencapture"]
```

## Writing one

`evoke new <name>` writes a working reflex to start from. `evoke check` reads the manifest, loads the body, and
writes `reflex.d.ts`, the one file a body imports. Every reflex here follows
[the rules](https://evoke.build/manual/author/rules.html) of the manual, which starts at
[your first reflex](https://evoke.build/manual/author/first-reflex.html). Here, `npm ci` then `npm run check` and
`npm test` type-check every body and run the tests. Licence: [MIT](LICENSE). Issues and changes:
[evoke-build/evoke](https://github.com/evoke-build/evoke).
