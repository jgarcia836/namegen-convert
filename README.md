# namegen-convert

Random name generators for games and worldbuilding tools usually end up
described in one of two shapes: a small hand-editable text grammar (good for
writing by hand) or a JSON document (good for feeding into another program).
Every generator I've built or borrowed ends up needing both, and hand
translating one into the other is exactly the kind of thing that quietly
introduces typos - a dropped weight, a renamed category that one reference
still points at. This is a converter between the two, so the text version can
stay the editable source of truth and the JSON version can be generated
instead of maintained by hand.

## The two formats

`.ngt` - plain text, meant to be edited directly:

```
!start name

first = Anna, Beth:2, Clara
last = Smith, Jones
name = {first} {last}
```

`.ngj` - the same document as JSON:

```json
{
  "start": "name",
  "categories": {
    "first": ["Anna", { "text": "Beth", "weight": 2 }, "Clara"],
    "last": ["Smith", "Jones"],
    "name": ["{first} {last}"]
  }
}
```

A category is a list of weighted text entries (default weight 1, written as
`:2` for a weight of 2 in `.ngt`, or `"weight": 2` in `.ngj`). An entry may
reference another category with `{other_category}`. `start` says which
category is the entry point for name generation; if it's left out, a category
literally called `name` is used instead.

The primary job of this tool is converting between the two representations,
but it can also expand a grammar's `start` category into generated names
itself, for trying out a draft without wiring it into a separate generator.

## Usage

```
namegen-convert [--lenient] [--from ngt|ngj] [--to ngt|ngj] <input> <output>
namegen-convert sample [--lenient] [--from ngt|ngj] [--count N] <input>
```

Format is normally inferred from the file extension (`.ngt`/`.txt` and
`.ngj`/`.json`). Use `--from`/`--to` to override that, for example when
reading from a path without a recognized extension.

```
namegen-convert grammar.ngt grammar.ngj
namegen-convert grammar.ngj grammar.ngt
```

## Strict by default

By default the converter refuses to produce output if the input has any of:

- a `start` category that doesn't exist
- a `{placeholder}` referencing a category that isn't defined
- the same category defined twice
- no way to determine a start category at all

These are almost always mistakes, and silently guessing what was meant tends
to produce a grammar that looks fine and generates garbage. Pass `--lenient`
to convert anyway: duplicate categories get merged, an unresolvable `start`
falls back to the first category defined, and unresolved placeholders are
left as literal text in the output. Every downgraded issue is still printed
as a warning on stderr, so `--lenient` doesn't mean silent.

```
namegen-convert --lenient rough-draft.ngt rough-draft.ngj
```

## Sampling

`sample` parses a grammar, resolves its `start` category the same way a
conversion would (strict by default, `--lenient` to downgrade the same
issues to warnings), and prints one generated name per line by picking a
random weighted entry from `start` and recursively expanding any
`{placeholder}` references it contains:

```
namegen-convert sample grammar.ngt
namegen-convert sample --count 5 grammar.ngj
```

Sampling is bounded to a fixed recursion depth, so a grammar with a
reference cycle (`a = {b}` / `b = {a}`) fails with an error instead of
hanging. There is no `--seed` flag yet, so runs are not reproducible.

## Building

Standard library only, no external crates:

```
cargo build --release
```

## Status

Early skeleton: the two formats convert both ways, strict/lenient validation
works, and `sample` can expand a parsed grammar into names. `sample` has its
own unit tests, and round-trip tests check that `.ngt` and `.ngj` fixtures
for the same grammar parse to the same document and survive being converted
to the other format and back - see the roadmap in the issue tracker.
