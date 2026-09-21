# Projects

A project is a directory of files you own. `evoke` reads the nearest one and writes only inside it.

```text
~/.config/evoke/   or   <app root>/
├── evoke.toml            yours: the adapter, the reflexes, their settings
├── evoke.lock            written by add, update and remove: what exactly is installed
├── evoke.d.ts            written with the lock: types for an app's decisions
├── overlays/lights.toml  yours: your wording for one reflex, named by its local name
└── vocab/rooms.toml      yours: your words
```

## Which project

The CLI uses the nearest `evoke.toml` upward from the working directory, else your **home project** in
`~/.config/evoke/` (or `$XDG_CONFIG_HOME/evoke/`). First use writes the home project with its one line,
`adapter = "jev"`. Any project but home must be [trusted](installing.md#trust) before `evoke` decides in it.

The SDK takes an explicit root and never searches: [SDK projects](../sdk/projects.md).

## `evoke.toml`

```toml
adapter = "jev"                    # who decides; a name, never a path; always stated

[reflexes]                         # local name = where it comes from
lights = "radhi/home/lights"       # unpinned: update moves it forward
timer  = "radhi/timer@1.0.1"       # pinned
hello  = "./hello"                 # local, relative to this file: never locked

[config.lights]                    # what the reflex's [config] declares
bridge = "10.0.0.2"                # a value, stored plain
token  = { env = "HUE_TOKEN" }     # a secret: the variable, never the value

[adapters.jev]                     # everything engine-specific, under the adapter's name
gate = { write = 0.85 }
```

- A **local name** matches `[a-z][a-z0-9_]*`; `none`, `unstated` and `fits` are reserved. It is what the
  classifier reads and what the overlay file is named after.
- An adapter name resolves only against `evoke`'s built-ins — never from the project directory, since an adapter
  sees every input. A table for an adapter not selected is inert, so one file serves devices on different engines.
- Edit the file by hand if you like: `evoke` validates it whole and names the line to fix. Its own writes keep your
  comments and order.

## Safe for dotfiles

A project holds only what you wrote, the lock and generated types — no fetched code, no secrets, no cache. Commit
it. On a new machine, `evoke sync` rebuilds the store from the lock and records the runtime; decisions run the same.

Machine-local state lives outside the project, under XDG: the store and the answer cache in `~/.cache/evoke/`; the
log, the trust file, the runtime path and the REPL's history in `~/.local/state/evoke/`. The full list:
[Environment](../reference/environment.md).

## An application's project

An app keeps its project at its root and reads it with the SDK. `evoke add` beside the app writes the same three
files, and `evoke.d.ts` types the app's decisions by reflex. Overlays and vocabularies under that root tune the
app's wording exactly as your home project tunes yours — and the SDK can hand a vocabulary per call, so a
multi-tenant server compiles each tenant's words into their own plan.

## Local reflexes

`hello = "./hello"` names a reflex directory beside the file: an author's own, or one you keep out of git. It is
never locked, its content is hashed at run, and its effect is what its manifest says. The store is not involved.

## Trust, briefly

Trust binds a project's four owned paths to their content. `evoke`'s own writes re-bless it; any other change asks
for `evoke trust` again. Home is trusted by construction. Why this matters: [Security](../security.md).

**Next:** the [Author](../author/first-reflex.md) section, or the [Reference](../reference/cli.md).
