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

The CLI looks for the nearest `evoke.toml`, upward from the working directory. If there is none, it uses your
**home project** in `~/.config/evoke/` (or `$XDG_CONFIG_HOME/evoke/`). First use writes the home project with its
one line, `adapter = "jev"`. Any project but home must be [trusted](installing.md#trust) before `evoke` decides in
it.

The SDK takes an explicit root and never searches: [Projects in code](../sdk/projects.md).

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

- A **local name** matches `[a-z][a-z0-9_]*`. `none`, `unstated`, `fits` and `weave` are reserved. The local name is what
  the classifier reads, and the overlay file is named after it.
- An adapter name resolves only against `evoke`'s built-ins, never from the project directory. An adapter sees
  every input, so that matters. A table for an adapter you did not select is inert. So one file serves devices on
  different engines.
- Edit the file by hand if you like. `evoke` validates it whole and names the line to fix. Its own writes keep your
  comments and order.

## Safe for dotfiles

A project holds only what you wrote, the lock and generated types. No fetched code, no secrets, no cache. Commit
it. On a new machine, `evoke sync` rebuilds the store from the lock and records the runtime. Decisions run the
same.

Machine-local state lives outside the project, under XDG. The store and the answer cache are in `~/.cache/evoke/`.
The log, the trust file, the runtime path and the REPL's history are in `~/.local/state/evoke/`. The full list:
[Environment](../reference/environment.md).

## An application's project

An app keeps its project at its root and reads it with the SDK. `evoke add` next to the app writes the same three
files, and `evoke.d.ts` types the app's decisions by reflex. Overlays and vocabularies under that root tune the
app's wording, just as your home project tunes yours. The SDK can also compile the same project over a tenant's
own vocabularies, with `with({ vocab })`. So a multi-tenant server keeps each tenant's words in their own plan.

## Local reflexes

`hello = "./hello"` names a reflex directory next to the file. It can be an author's own, or one you keep out of
git. It is never locked. Its content is hashed at run, and its effect is what its manifest says. The store is not
involved.

## Trust, briefly

Trust binds a project's four owned paths to their content. `evoke`'s own writes renew it. Any other change asks
for `evoke trust` again. Home is trusted by construction. Why this matters: [Security](../security.md).

**Next:** the [Author](../author/first-reflex.md) section, or the [Reference](../reference/cli.md).
