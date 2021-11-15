# List of Todos and other Issues

### Feature implementation status

| *Feature*       | *Readable* | *Writeble* |
|-----------------|------------|------------|
| file system     |    yes     |            |
| http request    |    yes     |            |
|                 |            |            |
| plain text      |            |            |
| markdown        |            |            |
|                 |            |            |
| json            |    yes     |            |
| yaml            |    yes     |            |
| toml            |    yes     |            |
| xml             |    yes     |            |
|                 |            |            |
| rust            |    yes     |            |
| python          |            |            |
| javascript      |            |            |
| go              |            |            |



| *Feature*       | *Support* |
|-----------------|-----------|
| path lang       |  partial  |
|                 |           |
| C interop       |           |
| Python interop  |           |


### Todos, Issues, Problems

- todo: add domain and trace traits, with full support in implementations
- todo: write support: json, rust, fs
- todo: c interop and a small c test

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

- todo: make ^http^rust work, allow http to function as string (auto conversions)

- unclear: file value should be the file name or file contents?

- unclear: we should have some internal language:
    - Usecase: json:  `/question[/answer_entities/*.is_empty()].count()`

- unclear: how to build a tree of results (what is the accepted language?)
```
    './**[.name=='config.yaml'][as composefile]^yaml/services/*/image[^string^http@status/code!=200]
    tree 'result' -> [composefile] -> image
```
