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

This tool only converts between the two representations - it does not itself
expand `{placeholders}` into generated names. That's a separate problem
(sampling, weighting, avoiding infinite recursion on cyclic grammars) and
belongs in a generator, not a format converter.

## Usage

```
namegen-convert [--lenient] [--from ngt|ngj] [--to ngt|ngj] <input> <output>
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

## Building

Standard library only, no external crates:

```
cargo build --release
```

## Status

Early skeleton: the two formats convert both ways and strict/lenient
validation works. No name sampling yet, no test suite yet - see the roadmap
in the issue tracker.
