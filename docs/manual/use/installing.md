# Installing reflexes

Git is the registry. A reflex lives in a repository, at a version tag. `evoke add` fetches it, pins it in a lock,
and keeps its code in a store keyed by content. It runs the code only while it still hashes to the lock. Nothing is
installed that a person did not name. No install-time code ever runs.

## Refs

| Form                              | Means                                                                              |
| :-------------------------------- | :--------------------------------------------------------------------------------- |
| `owner/repo`                      | Every reflex directory in the repository, on GitHub                                |
| `owner/repo/dir`                  | One reflex, the directory `dir` of the repository; `dir` may nest                  |
| `owner/repo/dir@1.2.0`            | Pinned: `update` keeps it there. Tags are `X.Y.Z` or `vX.Y.Z`. The newest is the highest |
| `https://host/repo.git#dir@1.2.0` | Any git host over `https` or `ssh`. `#dir` and `@tag` are optional                  |
| `ssh://git@github.com/owner/repo` | The same over ssh, the user before the host                                        |
| `./dir`, `../dir`                 | A local directory, relative to `evoke.toml`. Never fetched, never locked            |

An unpinned ref means its newest tag at `add`, and `update` moves it forward. A repository with no version tag
cannot be installed. Its author publishes one with `git push --tags`. A password or a token in a URL is refused:
git's credential helper holds those, never a project file.

## `add`

```text
$ evoke add radhi/home/lights radhi/timer
+ lights  radhi/home/lights 1.2.0  write  runs lights.mts
+ timer   radhi/timer 1.0.1        write  runs timer.mts
  inactive  lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"
  inactive  lights: config "bridge" is not set   →  evoke config lights bridge <value>
  inactive  lights: config "token" is not set    →  evoke config lights token --env <VAR>
```

A reflex whose manifest declares what its body touches shows it on a second line under its row, `needs writes
{to} ~/Downloads · hosts *`, as [the first ten minutes](../start/first-run.md#2-install-the-collection) show; a
row without one declares nothing.

`evoke add <ref>… [--as <name>]` does, in order:

1. **Fetches** each ref at its pin or its newest tag. This is a bare, shallow fetch, read without a checkout,
   once per repository and tag. Symlinks and submodules are refused. The tree is kept in the store. A local ref,
   `./dir`, is read where it is and written to `evoke.toml` as a path relative to that file. It is never fetched
   and never locked.
2. **Reads and lints** every manifest. A manifest that does not parse refuses the whole add. Nothing is written
   until every newcomer is in hand. Lint only reports; it never refuses. It flags a summary over 100 characters, a
   description over 1 000, more than eight `not_for` entries, more than 24 options, more than 40 records per
   table, an utterance over 200 characters, or text that addresses a model instead of describing an action.
3. **Tests for theft.** The examples already installed are routed over the new set, a few at a time. A phrase a
   newcomer wins prints as `<thief>: steals "<phrase>" from <owner>  →  evoke teach "<phrase>" not <thief>`. The
   add still proceeds. The fix is one line in your overlay. When the classifier cannot answer, or has no key yet,
   the line says the test did not finish, and `evoke test` runs it again.
4. **Writes** the `[reflexes]` lines to `evoke.toml`, the lock, and `evoke.d.ts`. It records the JavaScript runtime
   when a newcomer runs a file. Then it prints one row per newcomer behind `+`, then the lint lines, the theft
   lines, and each newcomer's inactive lines.

The **local name** is the ref's last segment. `--as <name>` picks another, for a single ref. The local name is
what the classifier reads, and your overlay file is named after it. A name already taken is refused. For one ref,
the fix is the `--as` line; for a collection, the line that adds the rest.

## `remove`

`evoke remove <name>` drops the `[reflexes]` line and the lock entry, and prints the row behind `-`. Your overlay,
your vocabularies and your settings stay, because they are yours. The store's copy stays too; it is a cache.

## `update`

```text
$ evoke update
  lights 1.2.0 → 2.0.0  major · code changed
    description  rewritten, taken (you have no override)
    args.state   renamed power; your 1 example follows
```

`evoke update [<name>]` moves every unpinned remote reflex, or just one, to its newest tag. A pinned reflex moves
to its pin. It **never prompts, never blocks, and never rewrites your files**. When nothing moves, it prints
`up to date`. A reflex whose new manifest does not read is reported at its new tree in the store and skipped; the
rest move, and the exit is 3. For each move, it prints the level of the change and one line per detail:

| Line                                  | Means                                                                          |
| :------------------------------------ | :----------------------------------------------------------------------------- |
| `same`, `minor`, `major`              | Wording only, or a declaration narrowed; additions, a declaration widened among them; something your files or calls may not survive |
| `· code changed`                      | A file other than the manifest changed                                         |
| `description  rewritten, taken`       | You have no override, so upstream's new text is used. Otherwise: `yours kept`  |
| `args.state  renamed power; your 1 example follows` | The author declared the rename; your records follow it at merge time |
| `stale`                               | You override something upstream changed: yours wins, both are shown once       |
| `orphaned`                            | A line of yours addresses nothing any more: skipped, the rest applies          |
| `effect  tightened to write`          | Upstream tightened the effect; it applies                                      |
| `effect  write upstream; destructive kept until you accept` | Upstream loosened it; you keep what you consented to  |
| `needs  narrowed: hosts * dropped`    | Upstream narrowed what the body touches; it applies                            |
| `needs  widened: writes ~/notes upstream; none kept until you accept` | Upstream widened it; you keep what you consented to |

The lock holds the effect and the declaration you consented to. Upstream may tighten either at any time. They
loosen only through `evoke update --accept <name>`, which takes the looser effect and the wider declaration at
the current tag and says so: `effect  write accepted; was destructive`, `needs  writes ~/notes accepted; was
none`. A new reflex in a repository you already use is reported once, with its add line. `update` never adds
one for you.

## `sync`

On a new device, or in CI, you have the lock and the store is empty. `evoke sync` fetches each repository once,
at every locked tag it needs, checks that each tree hashes to the lock, and records the runtime. It prints a `+`
row per reflex placed, or `up to date`. It never changes the lock. A tag that moved or vanished is refused, with
`evoke update <name>`. Before `sync`, `evoke show` lists what the lock names, each missing tree as an inactive
line. A git call that has not finished within a minute is a failure, never a hang.

```text
$ evoke sync
+ lights  radhi/home/lights 1.2.0  write  runs lights.mts
```

## `trust`

Your home project is trusted by construction. Any other project, an application's or a clone's, must be trusted
before `evoke` decides in it. A project's files decide what runs:

```text
$ cd app && evoke "kill the lights in the den"
  ~/app is not trusted  →  evoke trust
[3]
$ cd app && evoke trust
+ trusted ~/app
```

Trust binds to the content of the four owned paths: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`. `evoke`'s
own writes renew it. Any other edit, yours or a `git pull`'s, makes the next run say `changed since you trusted
it  →  evoke trust`. Trust lives in `~/.local/state/evoke/trust.toml`, on this machine only.

## The lock and the store

`evoke.lock` records, for each remote reflex, the ref, the tag, the commit, the content hash `h1` of the reflex
directory, and the effect and the declaration you consented to. It also records the adapter the project decides
with, by name and id.
Commit it with your project.

Fetched code lives under `~/.cache/evoke/store/<h1>/`. It is re-hashed against the lock every time `evoke` starts.
A store entry that does not match is a miss, and `evoke sync` places it again. Your project directory never holds
code you did not write.

## The runtime

A reflex that runs a `.mts` or `.mjs` file needs Node 24 or newer. `add` and `sync` find `node` on your `PATH`
and record its path in `~/.local/state/evoke/runtime`. Nothing is searched at run time, so a cron job's `PATH`
cannot break a decision. If none is found and none is recorded, the command ends with
`node is not on PATH; a .mts reflex needs Node 24 or newer  →  evoke sync` and exit 3, after its rows. Install
Node, and `evoke sync` records it. An argv reflex needs no runtime.

**Next:** [Tuning](tuning.md).
