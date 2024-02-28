# List of Todos and other Issues

## TODOs

! focus on releasing a first minimal version, then improve
    - interpretations: path+fs, json+yaml+toml+xml, rust+js, url?+http
    - explicit and implicit write support (policy, include readonly)
    - fix tests, todo!() and TODO: in code

- support type selector: `hial './src/tests/rust.rs^rust/:function_item'`
- support rust/ts write: `hial './src/tests/rust.rs^rust/:function_item[-1]#label = "modified_fn_name"'`
- set value on the command line
- separate api module, used by ffi and dependent crates

- operations:
    - assign to variables;
    - search with assignment of results
    - pretty print of variables/results
    - write values/trees to variables/results
        - write to cell (value, label and serial)
        - set index (write cell#index)
        - new/append/insert_at/delete cell
        - new/set/replace/delete group (only sub or attr group)
    - diff with assignment of results in variables

- ?treesitter representations are too detailed, unsure what to do
- ?explore python implementation and usage
- ?search should return all matches embedded in a delegation cell, which has all results
    as subs and delegates write operations to all the subs
- ?rename XCell, Cell, CellTrait to Nex?
- later: python, git, database, ical, zip, markdown


### Feature implementation status

| *Feature*  | *Readable* | *Writeble* |
|------------|------------|------------|
| url        |    yes     |    yes     |
| path       |    yes     |    yes     |
| fs         |    yes     |    yes     |
| http       |    yes     |    yes     |
| json       |    yes     |    yes     |
| yaml       |    yes     |    yes     |
| toml       |    yes     |    yes     |
| xml        |    yes     |    yes     |
| rust       |    yes     |            |
|            |            |            |
| git        |            |            |
| database   |            |            |
| ical       |            |            |
| zip        |            |            |
|            |            |            |
| plain text |            |            |
| markdown   |            |            |
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

- todo: c interop and a small c test
- cell must implement partialeq, eq (same pointed location)
- todo CLI:
    - todo: colors: interp, type, label, value
    - todo: option to hide attrs?

- todo: python interop and a larger python example
- todo: get should return an iterator; multiset labels
- todo: add regex operator and shortcuts for startswith, endswith, contains
- todo: add <, >, <=, >= operators
- todo: improve nom parsing errors, use context
- todo: interpretations parameters
- todo: custom tree datastructure?
- todo: cell symlinks
- todo: cell path
- todo: path bindings
- todo: diffs

- unclear: we should have some internal language:
    - Usecase: json:  `/question[/answer_entities/*.is_empty()].count()`

- unclear: how to build a tree of results (what is the accepted language?)
```
    './**[.name=='config.yaml'][as composefile]^yaml/services/*/image[^string^http@status/code!=200]
    tree 'result' -> [composefile] -> image
```
