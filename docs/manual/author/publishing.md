# Publishing

Publishing is a git tag. A reflex's identity is its location, `owner/repo[/dir]`. Its version is the tag it was
fetched at. There is no account, no registry API, no build step, and no upload.

## A repository of reflexes

```text
radhi/home                        a repository is a collection
├── lights/{reflex.toml, lights.mts, reflex.d.ts}
└── blinds/{reflex.toml, blinds.mts, reflex.d.ts}
```

- `evoke add radhi/home` installs every directory holding a `reflex.toml`. `evoke add radhi/home/lights` installs
  one. Directories may nest.
- A repository may also be a single reflex at its root. An author may keep a project inside that directory:
  `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`, `evoke.d.ts`. None of it ships. Those five names are skipped
  when a reflex is fetched and hashed.
- Symlinks and submodules are refused at fetch. Everything a body needs is a committed file in its directory.
- A `LICENSE` at the listed root is needed for a Hub listing. The collection uses MIT, since its scripts are meant
  to be copied into reflexes of your own.

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
+ reflex.d.ts
  1.2.0 → 2.0.0  major
    args.state  renamed power
```

| Level   | When                                                                                             |
| :------ | :----------------------------------------------------------------------------------------------- |
| `same`  | Wording only: description, `not_for`, `tags`, `confirm`, any `ask`, option meanings, records     |
| `minor` | Additions, like an argument, an option or a config key. Also a config key removed                |
| `major` | Something a user's files or calls may not survive: an argument or option removed or renamed, a source or range changed, `run` changed |

A `was` violation is refused outright. That means a retired name returning, or a name dropped from the list.
With no repository, no tag, or no reflex at this directory in the tag, nothing prints.

## Renaming safely

To rename an argument, declare its former names and keep them forever:

```toml
[args.power]
was = ["state"]
```

Every user's overlay and call follows at merge time, and `update` tells them so. To change an option key, add the
new key and keep the old one. Removing a key is `major`.

## What users see at `update`

`evoke update` never prompts, never blocks and never rewrites a user's files. It reports your rewritten
description as *taken*, unless they overrode it. It reports each contract change, and what they override that you
changed. It reports a loosened effect, which they keep until they accept it. A tightened effect applies at once.
Write the change so that this report reads well.

## What `add` checks, so write for it

Each install lints the manifest. It also routes the installed examples over the new set, and names every phrase
your reflex would steal from one already there. Distinct summaries, full `not_for` lists, and examples that sound
like real sentences keep your reflex out of that report.

## The Hub

The Hub is where reflexes are found. Until about twenty third-party reflexes exist, it is the first-party
collection, `evoke-build/reflexes`, reviewed and published as one repository. After that, it becomes one index
repository. A listing is a one-line pull request. Static pages at `evoke.build` show a preview, a diff, and a
search over examples. No install ever depends on it. `add` and `sync` reach git directly.

The Hub publishes no per-engine pass rates. An engine's terms may forbid them.

**Next:** [The collection](../collection.md), or the [SDK](../sdk/getting-started.md).
