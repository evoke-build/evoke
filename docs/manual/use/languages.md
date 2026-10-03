# Languages

<!-- description: Which languages evoke reads a request in, how it tells, what stays as typed, and what why says. -->

`evoke` reads a request in English, German, French or Spanish. Each language is a pack of words built into `evoke`:
the words that join two steps, the ones that ask for nothing, the pronouns that point back, the numbers, the days
and months, the clock, the units and the currencies, the signs said aloud. There is nothing to install and nothing
to set.

## How it tells

The words of the request show the language. A request holding more of one pack's own words than of any other's is
read by that pack, and by English always; one that mixes two languages, `Zeig mir die Sessions von Ana, por favor`,
is read by both where they show alike. A request so short that no pack shows, `wipe C02TM6PNHV29`, is read by every
pack, each holding back its two-letter words and the words English knows. A code, an address or a link shows no
language.

## What stays as typed

A value is always one of your own words, in your own language: a day, a number, a length of time, an amount, a time
of day said in German is read as the value the program takes, and a text — a label, a note, a message — is kept as
typed, never translated. The questions `evoke` asks you, its prompts and `why`'s lines stay English in this release.

At a confirm prompt a yes or a no in any of the four languages does: `y`, `yes`, `ja`, `oui`, `sí` run; `n`, `no`,
`nein`, `non` decline ([Outcomes](outcomes.md#confirm)).

## What `why` says

`evoke why` and `evoke try` name the language that read the request when it was not English, with the words that
showed it:

```text
  read by                 German, shown by «für», «den»
```

An English request says nothing of the kind. `evoke calibrate` and `evoke test` print a line per language other
than English that read a record, with how its records fared ([Calibrating](calibrating.md), [Tuning](tuning.md)).

## What a change to a pack does

A plan file carries the digest of every pack it was read with. A release that changes a pack refuses a plan file
made under the old one as stale, as it does when a question's wording changes, and `evoke run` makes the plan again
([Weaving](weaving.md)).
