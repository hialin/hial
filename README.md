
# Hial

Hial is a general purpose data API and CLI tool. It is a programmatic interface to different types of data represented as uniform tree structures. This makes the data easy to read, explore and modify using a small number of functions. Hial proposes a relatively simple mental model, suitable for most use cases, which improves the user comfort and speed.

The types of data that can be supported by this API are the file system, configuration files (json, yaml, toml), markup files (xml, html), programs written in various programming languages, operating system configurations and runtime parameters, database tables and records, etc.

The path API can be seen as an generalization of the concepts behind xpath, json path, file system path, and other similar path languages. It is a concise way to express data searches common in programming and system administration.

:warning:  Hial is **currently under construction.** Some things don't work yet and some things will change.

### What can it do?

##### 1. Select or search for pieces of data in a structured way.

Print a value embedded in a json file or from a url that returns json or xml data:

```bash
hial './examples/productiondump.json^json/stacks/*/services'
hial 'http://api.github.com^http^json/rate_limit_url^http^json/resources/core'
hial 'http://www.phonetik.uni-muenchen.de/cgi-bin/BASRepository/oaipmh/oai.pl^http^xml'
```

<!-- Print all questions that have no answer entities in a json file:

```bash
hial './myfile.json^json/question[count(/answer_entities/*)==0]'
# 🚧 todo: functions: sum, count, min, max
``` -->

Print all services with inaccessible images in a Docker compose file:

```bash
# shell
hial './config.yaml^yaml/services/*[ /image^split[":"]/[0]^http[HEAD]@status/code>=400 ]'
# 🚧 todo: split interpretation (regex[( ([^:]*): )*]
```

```rust
// rust (native)
for service in Cell::from("./config.yaml").all("^yaml/services") {
    let image = service.to("/image");
    if image.to("^http[HEAD]@status/code") >= 400 {
        println!("service {} has an invalid image: {}",
            service.read().value()?,
            image.read().value()?
        );
    }
}
```

Print the structure of a rust file (struct, enum, type, functions) as a tree:

```bash
hial './src/tests/rust.rs^rust/**[#type^split["_"]/[-1]=="item"]/*[name|parameters|return_type]'
# 🚧 todo: search results as tree
# 🚧 todo: boolean filter combinator
```

##### 2. Modify data selected as above.

Change the default mysql port systemwide:
```bash
# shell
hial '/etc/mysql/my.cnf^fs[w]^ini/mysqld/port = 3307'
# 🚧 todo: assign operator
```

```bash
// rust
Cell::from("/etc/mysql/my.cnf")
    .to("^fs[w]^ini/mysqld/port")
    .write()
    .value(3307)?;
```

Change the user's docker configuration:
```bash
# shell
hial '~/.docker/config.json^json/auths/docker.io/username = "newuser"'
```
```rust
// rust
Cell::from("~/.docker/config.json")
    .to("^fs[w]^json/auths/docker.io/username")
    .write()
    .value("newuser")?;
```

##### 3. Copy pieces of data from one place to another.

Copy a string from some json object entry which is embedded in a zip file, into a rust string:

```bash
# shell
hial 'copy ./assets.zip^zip/data.json^json/meshes/sphere  ./src/assets/sphere.rs^rust/**[#type=="let_declaration"][/pattern=sphere]/value'
# 🚧 todo: support copy
# 🚧 todo: support zip
# 🚧 todo: /**[filter] should match leaves only
```

Split a markdown file into sections and put each in a separate file:

```bash
# shell
`hial 'copy  ./book.md^md/*[#type=="heading1"][as x]  ./{label(x)}.md'
# 🚧 todo: support copy
# 🚧 todo: support markdown
# 🚧 todo: support interpolation in destination
```

##### 4. Transform data from one format or shape into another.

Transform a json file into an xml file with the same format and vice versa:

```bash
hial 'copy  file.json^json^tree^xml  ./file.xml'
hial 'copy  file.xml^xml^tree^json  ./file.json'
# 🚧 todo: support copy
# 🚧 todo: support tree implementation and conversion
```

##### 5. Structured diffs

Compare two files in different formats and print the resulting diff tree:

```bash
hial 'diff  ./file.json^json^tree  ./file.xml^xml^tree'
# 🚧 todo: support diff
# 🚧 todo: support tree implementation and conversion
```

Diff two diff trees (e.g. check if two different commits make identical changes)

```bash
hial 'x = diff .^git/HEAD^fs .^git/HEAD~1^fs ;
      y = diff .^git/branch1^fs .^git/branch1~1^fs ;
      diff $x $y
     '
# 🚧 todo: support diff
# 🚧 todo: support git interpretation
```

## Installation and usage

To test the examples or use the library from a shell, build the project: `cargo build --release`. Then run the `hial` command, e.g.: `hial 'http://api.github.com^http^json'`

## The data model

The data model is that of a tree of simple data nodes. The tree has a root node and a hierarchy of children nodes.

Each data node is called a **cell**. It may have a **value** (a simple data type like string, number, bool or blob). A cell is part of a **group** of cells. The cell may have an **index** (a number) or a **label** (usually a string) to identify it in this group.

A cell may have subordinate cells (children in the tree structure) which are organized into a **group**. We call this the **sub** group. A cell may also have attributes or properties which also cells and are put into the **attr** group. The children cells have the first cell as their **parent**.

A cell is always an **interpretation** of some underlying data. For example a series of bytes `7b 22 61 22 3a 31 7d` can have multiple interpretations:

1. a simple byte array which is represented by a single cell with the data as a blob value:
```
Cell: value = Blob([7b 22 61 22 3a 31 7d]),
```
2. a string of utf-8 encoded characters which is represented by a single cell with the data as a string value:

```
Cell: value = String("{\"a\":1}"),
```

3. a json object which is represented by a tree of cells, the root cell being the json object `{}` with a sub cell with label `a` and value `1`:

```yaml
Cell:
    type: "object",
    sub:
        Cell:
            label: "a",
            value: 1,
            type: "number",
```

Usually a piece of data has a humanly obvious best interpretation (e.g. json in the previous example), but the data can be always explicitly reinterpreted differently.

A cell also has a string **type** describing its kind, depending on the interpretation. Such types can be: "file" or "folder" (in the *fs* interpretation), "array" (in the *json* interpretation), "function_item" (in the *rust* interpretation), "response" (in the *http* interpretation), etc.

````mermaid
---
title: Data model diagram
---
erDiagram
    Cell 1--o| "Sub Group" : "sub()"
    Cell 1--o| "Attr Group" : "attr()"
    Cell {
          int index
          value label
          value value
          string type
    }
    "Sub Group" 1--0+ "Cell" : "at(), get()"
    "Attr Group" 1--0+ "Cell" : "at(), get()"
````

#### Examples:

- A *folder* of the file system is a cell. It has a *sub* group and may have *sub* cells (files or folders which it contains); it may also have a *parent* cell (parent folder). Its *attr* items are creation/modification date, access rights, size, etc. The folder name is the *label* and has no *value*.

- A *file* of the file system is a cell. It has no *sub* items, may have one *parent*, has the same *attr* as a folder and the *label* as its name. A file cell can be *interpreted* in many other ways (string cell, json/yaml/xml cell tree, programming cell trees).

- An entry into a json object is a cell. The json key in the key/value pair is the cell *label*. If the value of this json object entry is null or bool or number, then the cell will have a corresponding value and no *sub*; if it's an array or object then the cell will have a *sub* group with the content of the array or object.

- A method in a java project is a cell. It has a parent class, access attributes (*attr*), and arguments, return type and method body as children (*sub*).

- An http call response is a cell. It has status code and headers as *attr* and the returned body data as its value (a blob). It is usually further interpreted as either a string or json or xml etc.

### Path language

This unified data model naturally supports a path language similar to a file system path, xpath or json path. A cell is always used as a starting point (e.g. the file system current folder). The `/` symbol designates moving to the *sub* group; the `@` symbol to the *attr* group. Jumping to a different interpretation is done using the `^` (elevate) symbol.

As a special case, the starting point of a path is allowed to be a valid url (starting with `http://` or `https://`) or a file system path (which must be either absolute, starting with `/`, or relative, starting with `.`).

Other special operators are the `*` operator which selects any cell in the current group and the `**` operator which selects any cell in current group and any cell descendants in the current interpretation. Filtering these cells is done by boolean expressions in brackets.

Examples:

- `.^fs` is the current folder ("." in the file system interpretation). It is equivalent to just `.`.
- `./src/main.rs` is the `main.rs` file in the ./src/ folder.
- `./src/main.rs@size` is the size of this file (the `size` attribute of the file).

- `./src/main.rs^rust` represents the rust AST tree.
- `./src/main.rs^rust/*[#type=='function_item']` are all the top-level cells representing functions in the `main.rs` rust file.

- `http://api.github.com` is a url cell.
- `http://api.github.com^http` is the http response of a GET request to this url.
- `http://api.github.com^http^json` is the json tree interpretation of a http response of a GET request to this url.
- `http://api.github.com^http^json/rate_limit_url^http^json/resources/core/remaining` makes one http call and uses a field in the respose to make another http call, then selects a subfield in the returning json.

- `./src/**^rust` returns a list of all rust files (all files that have a rust interpretation) descending from the `src` folder.
- `./src/**^rust/**[#type=="function_item"]` lists all rust functions in all rust files in the `src` folder.
- `./src/**^rust/**[#type=="function_item"]/**[#type=="let_declaration"]` lists all occurences of *let* declarations in all functions in all rust files in the src folder.
- `./src/**^rust/**[#type=="function_item"]/**[#type=="let_declaration"][/pattern/*]` lists only destructuring patterns in all occurences of *let* declarations in all functions in all rust files in the src folder. The destructuring patterns are the only ones that have a descendant of `pattern`, for which the filter `[/pattern/*]` is true.

## What's the current project status?

See [status.md](doc/status.md) and [issues.md](doc/issues.md).

The implementation language is Rust, and a Rust API is natively available.

As a command line tool hial can be used from any language that can call shell commands.

C, Python, Java, Go, Javascript wrappers are planned.
