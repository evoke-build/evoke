# evoke-build/reflexes

The first-party reflexes for [evoke](https://github.com/evoke-build/evoke): what a Mac does at a word. The word
is decided by [Jev](https://typesafe.ai), TypeSafe AI's classifier. Each reflex is one directory, with
`reflex.toml` and the file it runs, where it runs one. There is nothing to build, and nothing to install but
`evoke` itself.

```bash
evoke add evoke-build/reflexes
```

| Reflex                               | Does                                                          | Effect      | Yours to set     |
| :----------------------------------- | :------------------------------------------------------------ | :---------- | :--------------- |
| [awake](awake/reflex.toml)           | Keeps the laptop awake for a duration, or until `pkill caffeinate` | write  |                  |
| [download](download/reflex.toml)     | Saves a URL's file to `~/Downloads`, or to a place you name   | write       | `places`         |
| [lock](lock/reflex.toml)             | Locks the screen                                              | write       |                  |
| [mail](mail/reflex.toml)             | Starts an email in your mail app                              | read        |                  |
| [note](note/reflex.toml)             | Appends a dated line to your notes file                       | write       | `file`           |
| [open](open/reflex.toml)             | Opens one of your folders                                     | read        | `places`         |
| [power](power/reflex.toml)           | Sleeps, restarts or shuts down                                | destructive |                  |
| [screenshot](screenshot/reflex.toml) | Captures the screen, a window or a selection                  | write       |                  |
| [timer](timer/reflex.toml)           | Counts down, then rings                                       | write       |                  |
| [trash](trash/reflex.toml)           | Empties the trash                                             | destructive |                  |
| [visit](visit/reflex.toml)           | Opens one of your sites, in a private window on request       | read        | `sites`          |
| [volume](volume/reflex.toml)         | Sets the output volume                                        | write       |                  |
| [wifi](wifi/reflex.toml)             | Turns Wi-Fi on or off                                         | write       |                  |

## Yours to set

A reflex that reads your words or a setting stays inactive until it has them. `evoke` says which line gives it
what it needs. `places` names your folders, and each word's value is its full path. `sites` names your sites, and
each value is its URL.

```bash
evoke vocab places add desktop "The desktop." --value /Users/you/Desktop
evoke vocab sites add github "GitHub." --value https://github.com
evoke config note file ~/notes.txt
```

## Writing one

`evoke new <name>` writes a working reflex to start from. `evoke check` reads the manifest, loads the body, and
writes `reflex.d.ts`, the one file a body imports. Here, `npm ci` then `npm run check` and `npm test` type-check
every body and run the tests. Licence: [MIT](LICENSE). Issues and changes:
[evoke-build/evoke](https://github.com/evoke-build/evoke).
