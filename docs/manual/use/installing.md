# Installing reflexes

Git is the registry. A reflex lives in a repository at a version tag; `evoke add` fetches it, pins it in a lock,
keeps its code in a content-addressed store, and runs it only while it still hashes to the lock. Nothing is
installed that a person did not name, and no install-time code ever runs.

## Refs

| Form                              | Means                                                                              |
| :-------------------------------- | :--------------------------------------------------------------------------------- |
| `owner/repo`                      | Every reflex directory in the repository, on GitHub                                |
| `owner/repo/dir`                  | One reflex, the directory `dir` of the repository; `dir` may nest                  |
| `owner/repo/dir@1.2.0`            | Pinned: `update` keeps it there. Tags are `X.Y.Z` or `vX.Y.Z`; the newest is the highest |
| `https://host/repo.git#dir@1.2.0` | Any git host over `https` or `ssh`; `#dir` and `@tag` optional                      |
| `./dir`, `../dir`                 | A local directory, relative to `evoke.toml`; never fetched, never locked            |

Unpinned, a ref means its newest tag at `add`, and `update` moves it forward. A repository with no version tag
cannot be installed: its author publishes one with `git push --tags`.

## `add`

```text
$ evoke add radhi/home/lights radhi/timer
+ lights  radhi/home/lights 1.2.0  write  runs lights.mts
+ timer   radhi/timer 1.0.1        write  runs timer.mts
  inactive  lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"
  inactive  lights: config "bridge" is not set   →  evoke config lights bridge <value>
  inactive  lights: config "token" is not set    →  evoke config lights token --env <VAR>
```

`evoke add <ref>… [--as <name>]` does, in order:

1. **Fetches** each ref at its pin or its newest tag — a bare shallow fetch read without a checkout, symlinks and
   submodules refused — and keeps the tree in the store.
2. **Reads and lints** every manifest. A manifest that does not parse refuses the whole add; nothing is written
   until every newcomer is in hand. Lint only reports: a summary over 100 characters, a description over 1 000,
   more than eight `not_for` entries, 24 options, 40 records per table, an utterance over 200 characters, or text
   that addresses a model instead of describing an action.
3. **Tests for theft.** The examples already installed are routed over the new set; a phrase a newcomer wins prints
   as `<thief>: steals "<phrase>" from <owner>  →  evoke teach "<phrase>" not <thief>`. The add proceeds; the fix
   is one line in your overlay.
4. **Writes** the `[reflexes]` lines to `evoke.toml`, the lock, and `evoke.d.ts`; records the JavaScript runtime
   when a newcomer runs a file; prints one row per newcomer behind `+`, then the lint lines, the theft lines, and
   each newcomer's inactive lines.

The **local name** is the ref's last segment, or `--as <name>`, which goes with a single ref. It is what the
classifier reads and what your overlay is named after. A name already taken is refused with the `--as` line that
resolves it.

## `remove`

`evoke remove <name>` drops the `[reflexes]` line and the lock entry and prints the row behind `-`. Your overlay,
your vocabularies and your settings stay — they are yours — and so does the store's copy, which is a cache.

## `update`

```text
$ evoke update
  lights 1.2.0 → 2.0.0  major · code changed
    description  rewritten, taken (you have no override)
    args.state   renamed power; your 1 example follows
```

`evoke update [<name>]` moves every unpinned remote reflex — or one — to its newest tag, and a pinned one to its pin.
It **never prompts, never blocks, and never rewrites your files**. For each move it prints the level of the change
and one line per detail:

| Line                                  | Means                                                                          |
| :------------------------------------ | :----------------------------------------------------------------------------- |
| `same`, `minor`, `major`              | Wording only; additions; something your files or calls may not survive         |
| `· code changed`                      | A file other than the manifest changed                                         |
| `description  rewritten, taken`       | You have no override, so upstream's new text is used; `yours kept` otherwise   |
| `args.state  renamed power; your 1 example follows` | The author declared the rename; your records follow it at merge time |
| `stale`                               | You override something upstream changed: yours wins, both are shown once       |
| `orphaned`                            | A line of yours addresses nothing any more: skipped, the rest applies          |
| `effect  tightened to write`          | Upstream tightened the effect; it applies                                      |
| `effect  destructive upstream; write kept until you accept` | Upstream loosened it; you keep what you consented to  |

The lock holds the effect you consented to. Upstream may tighten it at any time; it loosens only through
`evoke update --accept <name>`, which takes the looser effect on at the current tag and says so. A new reflex in a
repository you already use is reported once with its add line; `update` never adds one for you.

## `sync`

On a new device, or in CI, the lock is what you have and the store is empty. `evoke sync` fetches each locked
reflex at its locked tag, checks that its tree hashes to the lock, and records the runtime. It prints a `+` row per
reflex placed and never changes the lock; a tag that moved or vanished is refused with `evoke update <name>`.

```text
$ evoke sync
+ lights  radhi/home/lights 1.2.0  write  runs lights.mts
```

## `trust`

Your home project is trusted by construction. Any other project — an application's, a clone's — must be trusted
before `evoke` decides in it, because a project's files decide what runs:

```text
$ cd app && evoke "kill the lights in the den"
  ~/app is not trusted  →  evoke trust
[3]
$ cd app && evoke trust
+ trusted ~/app
```

Trust binds to the content of the four owned paths — `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`. `evoke`'s
own writes re-bless it; any other edit, yours or a `git pull`'s, makes the next run say `changed since you trusted
it  →  evoke trust`. Trust lives in `~/.local/state/evoke/trust.toml`, on this machine only.

## The lock and the store

`evoke.lock` records, per remote reflex, the ref, the tag, the commit, the content hash `h1` of the reflex
directory, and the effect you consented to; and the adapter the project decides with, by name and id. Commit it
with your project.

Fetched code lives under `~/.cache/evoke/store/<h1>/` and is re-hashed against the lock before every run. A store
entry that does not match is a miss: `evoke sync` places it again. Your project directory never holds code you
did not write.

## The runtime

A reflex that runs a `.mts` or `.mjs` file needs Node 24 or newer. `add` and `sync` find `node` on your `PATH` and
record its path in `~/.local/state/evoke/runtime`; nothing is searched at run time, so a cron job's `PATH` cannot
break a decision. None found and none recorded ends the command with `evoke sync`, exit 3, after its rows. An argv
reflex needs no runtime.

**Next:** [Tuning](tuning.md).
