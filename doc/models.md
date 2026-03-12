# Models of various domains

## Path

x
    dir
    name
    ext
    stem

Examples:

- `./src/main.rs^path/dir` -> `./src`
- `./src/main.rs^path/name` -> `main.rs`
- `./src/main.rs^path/ext` -> `.rs`
- `./src/main.rs^path/stem` -> `main`

Write examples:

- `./src/main.rs^path/ext = ".txt"` -> `./src/main.txt`
- `./src/main.rs^path/stem = "app"` -> `./src/app.rs`

## File system

x
    file
        @size
        @modification_time
        @access_time
        @creation_time
        @owner
        @group
        @permissions

## JSON

x
    key: value
    key: array
        value
        object
    key: object

## Text

x
    line
    line
    line

Examples:

- `./notes.txt^text/[0]` -> first line
- `./notes.txt^text/[1] = "updated"` -> rewrites the second line

# xml

x
    @version
    @encoding
    @standalone
    doctype
    pi
    element
        @attr1
        @attr2
        text
        element1
        element2
        comment
        cdata

# http

x
    @status_code
    @status_text
    @headers
    @body
