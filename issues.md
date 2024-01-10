# List of Todos and other Issues

## TODOs

- clean up rust representation
- explore python implementation and usage
- implement policy(): set on cells and propagated, not set by interpretations
- search should return all matches embedded in a delegation cell, which has all results
    as subs and delegates write operations to all the subs
- add json treesitter interpretation, compare it with rust outputs
- json: use SerdeValue directly instead of Node
- explicit domain save/write: to origin, to new domain
- write policies on domain (interpretation):
    - read only, write ignore, write back, write to new domain
- fix double kleene error (see test)
- fix extra::interpretation()
- fix todo!() and TODO: in code


### Feature implementation status

| *Feature*  | *Readable* | *Writeble* |
|------------|------------|------------|
| url        |    yes     |            |
| path       |    yes     |            |
| fs         |    yes     |            |
| http       |    yes     |            |
| markdown   |            |            |
| json       |    yes     |            |
| rust       |    yes     |            |
| git        |            |            |
| database   |            |            |
| ical       |            |            |
| zip        |            |            |
|            |            |            |
| plain text |            |            |
| yaml       |    yes     |            |
| toml       |    yes     |            |
| xml        |    yes     |            |
|            |            |            |
| python     |            |            |
| javascript |            |            |
| go         |            |            |
|------------|------------|------------|





| *Feature*       | *Support* |
|-----------------|-----------|
| path lang       |  partial  |
|                 |           |
| C interop       |           |
| Python interop  |           |


### Todos, Issues, Problems

- todo: write support: json, rust, fs
- todo: c interop and a small c test

- cell must implement partialeq, eq (same pointed location)

- todo CLI:
    - todo: tree guide lines
    - todo: colors: interp, type, label, value
    - todo: option to hide attrs?

- todo: python interop and a larger python example
- todo: review examples, check accessors, operators

- todo: get should return an iterator; multiset labels

- todo: add regex operator and shortcuts for startswith, endswith, contains
- todo: add <, >, <=, >= operators

- todo: improve nom parsing errors, use context
- todo: interpretations parameters
- todo: custom tree
- todo: cell symlinks
- todo: cell path
- todo: path bindings
- todo: diffs
- todo: git interpretation

- todo: make ^http^rust work, allow http to function as string (auto conversions?)

- unclear: file value should be the file name or file contents?

- unclear: we should have some internal language:
    - Usecase: json:  `/question[/answer_entities/*.is_empty()].count()`

- unclear: how to build a tree of results (what is the accepted language?)
```
    './**[.name=='config.yaml'][as composefile]^yaml/services/*/image[^string^http@status/code!=200]
    tree 'result' -> [composefile] -> image
```
