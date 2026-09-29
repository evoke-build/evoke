# Publishing

Publishing is a git tag. A reflex's identity is its location, `owner/repo[/dir]`. Its version is the tag it was
fetched at. There is no account, no registry API, no build step, and no upload.

## A repository of reflexes

```text
radhi/home                        a repository is a collection
├── lights/{reflex.toml, lights.mts, reflex.d.ts}
└── blinds/{reflex.toml, blinds.mts, reflex.d.ts}
```

- `evoke add radhi/home` installs every directory directly under it that holds a `reflex.toml`.
  `evoke add radhi/home/lights` installs one. Directories may nest: name a deeper directory, or the one above it.
- A repository may also be a single reflex at its root. An author may keep a project inside that directory:
  `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`, `evoke.d.ts`. None of it ships. Those five names are skipped
  when a reflex is fetched and hashed.
- Symlinks and submodules are refused at fetch. Everything a body needs is a committed file in its directory.
- A `LICENSE` at the listed root is needed for a Hub listing. The collection uses MIT, since its scripts are meant
  to be copied into reflexes of your own.

Design a collection before you word its reflexes: [rules 37 to 42](rules.md#8-how-to-design-a-collection).

## Tags and versions

```bash
git tag v1.2.0 && git push --tags
```

- Tags are `X.Y.Z` or `vX.Y.Z`. The newest is the highest. A repository without a version tag cannot be installed.
- **Published tags never change.** The lock records the tag, the commit and the content hash. `sync` refuses a tag
  that moved. Publish a fix as a new tag.
- One tag covers the whole repository. Every reflex in a collection moves together, and `evoke check` reads the
  contract diff for each from the same series.

## What the next version must be

Run `evoke check` in a reflex directory inside a git repository. It diffs the manifest's contract against the
repository's newest version tag, and says what the next tag must carry:

```text
$ evoke check
  lights  write  runs lights.mts
  lint  lights: args.power has no record that leaves it out; assert power = false once
  lint  lights: args.brightness has no record that leaves it out; assert brightness = false once
  lint  lights: examples holds 2 requests; write three at least
+ reflex.d.ts
  1.2.0 → 2.0.0  major
    args.state  renamed power
```

Each `lint` line reports a rule the manifest breaks. Lint never refuses:
[What `evoke` checks for you](rules.md#what-evoke-checks-for-you).

| Level   | When                                                                                             |
| :------ | :----------------------------------------------------------------------------------------------- |
| `same`  | Wording only: description, `not_for`, `tags`, `confirm`, any `ask`, option meanings, records. Also a declaration narrowed |
| `minor` | Additions, like an argument, an option, a config key, a field under `[yields]`, a `returns` name or a platform. Also a config key removed, an argument made optional, a secret made plain, or a declaration widened |
| `major` | Something a user's files or calls may not survive: an argument or option removed or renamed, a source, a range or `recent` changed, `run` or `steps` changed, an argument made required, a config key made secret, a yield or a `returns` name removed or changed, a taken argument added, removed or changed, a platform dropped |

A `was` violation is refused outright. That means a retired name returning, or a name dropped from the list.
With no repository, no tag, or no reflex at this directory in the tag, no diff prints.

Read this diff before every tag: [rule 46](rules.md#10-how-to-work). The [checklists](rules.md#checklists) list
what else to check before a tag.

## Renaming safely

`was` lists an argument's former names, which stay listed forever:

```toml
[args.power]
was = ["state"]
```

Every user's overlay and call follows at merge time, and `update` tells them so. Rename an argument with `was`:
[rule 46](rules.md#10-how-to-work). An option key has no `was`, and removing one is `major`.

## What users see at `update`

`evoke update` never prompts, never blocks and never rewrites a user's files. It reports your rewritten
description as *taken*, unless they overrode it. It reports each contract change, and what they override that you
changed. It reports a loosened effect, which they keep until they accept it. A tightened effect applies at once.

## What `add` checks

Each install lints the manifest, as `evoke check` does. It also routes every installed example over the new set.
A `steals` line names an example your reflex would take from the reflex that owns it. An `also fits` line names
an example your reflex also fits, well enough that the example would stop at confirm. For a playbook, `add` names
each step whose reflex lacks the playbook's tag. None of these lines stops the install.

Before a tag, install your reflexes beside the ones they will live with, and read every such line:
[rules 40 and 41](rules.md#8-how-to-design-a-collection).

## The Hub

The Hub is where reflexes are found. Until about twenty third-party reflexes exist, it is the first-party
collection, `evoke-build/reflexes`, reviewed and published as one repository. After that, it becomes one index
repository. A listing is a one-line pull request. Static pages at `evoke.build` show a preview, a diff, and a
search over examples. No install ever depends on it. `add` and `sync` reach git directly.

The Hub publishes no per-engine pass rates. An engine's terms may forbid them.

**Next:** [The collection](../collection.md), or the [SDK](../sdk/getting-started.md).
