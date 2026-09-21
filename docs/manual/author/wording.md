# Wording

The classifier is never trained. What it knows about your reflex is your manifest, turned into questions. So the
wording decides accuracy. This page is the craft of it.

## What the classifier sees

For one input, three kinds of question are asked, and each is built from your words:

| Question                         | Built from                                                                    |
| :------------------------------- | :---------------------------------------------------------------------------- |
| **Which reflex?** One choice over every installed reflex, plus *none of these* | Each reflex's `description` as *what*, its `not_for` as what it is not for, and its examples |
| **Does this reflex do what was asked?** A yes/no per reflex | `description` as *yes*, `not_for` as *no*                            |
| **Which value?** One choice per argument, plus *unstated* | The `ask`, each option's meaning, and the examples that assert it     |

Examples attach to the option they assert. `"kill the lights" = { state = "off" }` teaches two things: this input
is `lights`, and *kill* means `off`. A `false` example joins `not_for`.

## Describe the action, in the user's words

- The **summary** is the first line of `description`. Wide rankings see it. Say what the reflex does, plainly:
  *Turn the lights in one room on, off, or dim them.* Keep it under 100 characters.
- The rest of `description` draws the boundary: *Ceiling and lamp lights only.* What it covers, what it does to the
  world, what a person would need to know before saying yes.
- Never address the model. Lint names phrases like *Choose this when…*, *you must* and *ignore previous*. They
  also read worse than a plain description.

## Name the neighbours

`not_for` is where accuracy is won. List what a person might say that sounds like this reflex and is not:

```toml
not_for = ["keeping the laptop awake", "alarms at a time of day", "asking the time"]    # timer
not_for = ["timers and reminders", "putting the laptop to sleep", "screen brightness"]  # awake
```

Two reflexes installed side by side each carry the other in `not_for`. Then write the same neighbours as `false`
tests, so `evoke test` proves the boundary holds.

## Ask atomic questions

The classifier reads literally. Write one `ask` per argument, direct, as a person would ask it: *Which room?*,
*How loud, in percent?*, *The whole screen, a window, or a selection?*. Negations and two-part questions hurt. So
does a question that depends on another argument's answer.

Option meanings are answers to that question. Write one line each, and keep them distinct: *Suspend; wakes on
the lid or a key.* *Reboot.* *Power off.*

## Let picks and vocabularies do the reading

Numbers, durations, addresses, URLs and quoted phrases are found by code. The classifier only chooses among
them. So use a pick, not an option list, for anything open-ended. Words only the user knows, like rooms, folders
and sites, are a vocabulary. Name it by the convention the collection set, so that reflexes share one list.

## Be honest about the effect

`effect` decides how sure `evoke` must be before it runs unasked. `read` observes. `write` changes something a
person can undo. `destructive` cannot be undone, or costs something, and it always confirms. Absent, it is
destructive. One reflex has one effect. Grouped actions take the worst case, or become two reflexes.

## The confirm line

`confirm` is the question a person answers when `evoke` is not sure. Keep it short and specific. Name the
required arguments and nothing else: *Set the {room} lights {state}?*, *Really {action} now?*. It is the last
thing between a sentence and an action.

## Examples that teach

- Vary the verbs and the phrasing: *set a timer for 10 minutes*, *25 minute timer*, *count down 90 seconds*.
- Cover every option and every pick. Include an unstated case for what is optional.
- Keep them true. An example that asserts a wrong value teaches the wrong thing, and `evoke test` runs it.
- Jev reads English most accurately. A user's overlay carries any other language.

**Next:** [Publishing](publishing.md).
