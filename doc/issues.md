# List of Todos and other Issues

- color the output of pprint
    - maybe stream the found cells, colorize separately
- switch to anyhow errors

- add split(":") interpretation, read-write
- '**[filter]' must work as '**/*[filter]' (filter to be applied only on leaves)
- support rust/ts write: `hial './src/tests/rust.rs^rust/*[:function_item].label = "modified_fn_name"'`
- add interpretation params to Xell::be()
- support zip, markdown
- support 'copy source destination'
- support ^json^tree^xml
- support diff  ./file.json^json^tree  ./file.xml^xml^tree
- basic profiling
- functions
- should blobs/bytes be part of value? they are only useful via reinterpretation
- what to do with very large values? files which are 100MBs?

- release first minimal version:
    - interpretations: path+fs, json+yaml+toml+xml, rust+js, url?+http
    - explicit and implicit write support (policy, include readonly)
    - fix tests, todo!() and TODO: in code

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

- ?change search: multiple path indices for one cell
- ?treesitter representations are too detailed, unsure what to do
- ?explore python implementation and usage
- ?search should return all matches embedded in a delegation cell, which has all results
    as subs and delegates write operations to all the subs
- later: python, git, database, ical, zip, markdown


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
- todo: custom tree datastructure?
- todo: cell symlinks
- todo: path bindings

- unclear: we should have some internal language:
    - Usecase: json:  `/question[/answer_entities/*.is_empty()].count()`

- unclear: how to build a tree of results (what is the accepted language?)
```
    './**[.name=='config.yaml'][as composefile]^yaml/services/*/image[^string^http@status/code!=200]
    tree 'result' -> [composefile] -> image
```
