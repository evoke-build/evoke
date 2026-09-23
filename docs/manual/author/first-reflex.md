# Your first reflex

A reflex is a directory: a manifest and the file it runs. `evoke new` writes a working one. `evoke check` keeps it
honest. A project next to it lets you try it before anyone installs it.

## 1. Make it

```text
$ evoke new hello
+ hello/reflex.toml
+ hello/hello.mts
+ hello/reflex.d.ts
```

Three files, from a template that already works:

```toml
# hello/reflex.toml
reflex = 1

description = """
Say hello to someone.
A greeting with the name given, printed back."""
not_for = ["saying goodbye"]
effect = "read"
confirm = "Say hello to {who}?"
run = "hello.mts"

[args.who]
ask  = "Who should be greeted?"
pick = "quoted"

[examples]
'say hello to "Ada"' = { who = "Ada" }
'greet "Grace"'      = { who = "Grace" }

[tests]
'wave at "Linus"' = { who = "Linus" }
"what time is it" = false
```

```ts
// hello/hello.mts
import type { Reflex } from "./reflex.d.ts"

export default (async ({ who }) => `Hello, ${who}!`) satisfies Reflex
```

`reflex.d.ts` is generated from the manifest. It holds `Args`, with `who: string`, and the `Reflex` type that
checks the body's arguments and its return. The body imports that one file and nothing else.

## 2. Check it

```text
$ cd hello && evoke check
  hello  read  runs hello.mts
```

`evoke check` reads the manifest and names every line to fix. It confirms that the file `run` names exists,
loads, and exports a function by default. It reports lint. When the arguments changed, it rewrites `reflex.d.ts`
and prints `+ reflex.d.ts`. Inside a git repository with a version tag, it also diffs the contract against that
tag: [Publishing](publishing.md).

```text
$ evoke check
  broken: run: "nothing.ts" must end in .mts or .mjs  →  ~/broken/reflex.toml:4:1
[3]
```

## 3. Try it

A reflex is decided on inside a project. Keep one next to your reflexes while you work: a file naming the
adapter, and `evoke add ./hello` writes the rest.

```text
~/dev/
├── evoke.toml
└── hello/
```

```toml
# ~/dev/evoke.toml
adapter = "jev"
```

A project outside home must be trusted once. Edits to the reflex itself never need it again. Trust binds to the
project's own files, not to the reflexes.

```text
$ cd ~/dev && evoke trust
+ trusted ~/dev
$ evoke add ./hello
+ hello  ./hello  read  runs hello.mts
$ evoke try 'say hi to "Ada"'
  hello 0.94 · none 0.06
  who   "Ada" 0.97 · unstated 0.03
  fits  hello 0.71
  run · weakest: route 0.94
$ evoke 'wave at "Grace"'
  hello who="Grace"  0.92
Hello, Grace!
```

`evoke test hello` runs the manifest's own examples and tests against the classifier, and reports every miss.
`evoke run hello who=Ada` calls the body without the classifier at all.

## 4. Make it yours

From here, three pages carry the rest: the [manifest](manifest.md) key by key, the four kinds of
[argument](arguments.md), and what a [body](body.md) receives and returns. Then [wording](wording.md), which
decides accuracy more than anything else. And [publishing](publishing.md), which is a git tag.

**Next:** [The manifest](manifest.md).
