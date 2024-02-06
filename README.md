
# Hial

Hial is an uniform data API, particularly suitable for textual data. It is a CLI tool backed by a library.

An uniform data API is a programmatic interface to different types of data represented in an uniform manner (a general tree/graph structure). This makes the data easy to read, explore and modify using a small number of functions. The types of data that can be supported by this API are the file system structure, usual configuration files (json, yaml, toml), markup files (xml, html), programs written in various programming languages, operating system configurations and runtime parameters, database tables and records, etc.

### Why is it needed?

The uniform data API aims to make common data read/search/write operations easy to declare and execute. It maximizes user comfort and speed because it requires a single mental model, which is suitable for most use cases. A simple and uniform data model makes it easy to create, transform, filter and delete any kind of data, manually or programmatically.

The following tasks should be easy with such an API:
- pinpoint changes in configuration files (e.g. change a specific value in a json/yaml/toml/text file)
- interactive or automated data exploration (structured grep)
- semantic diffs
- custom program refactoring
<!-- - complex data conversions -->

## The data model

🚧 **Hial is currently under construction. Some things do not work yet and some other things will change**. 🚧

The data model is that of a tree of simple data nodes. The tree has a root node and a hierarchy of children nodes.

Each data node is called a **cell**. It may have a **value** (a simple data type like string, number, bool or blob).

A cell may have subordinate cells (children in the tree structure) which are organized into a **group**. We call this the **sub** group. A cell may also have attributes or properties which also cells and are put into the **attr** group.

All cells except the root cell have a **super** cell and are part of the **super** group (all the cells with the same parent, or the sub group of the super cell). A cell may have an **index** (a number) or a **label** (usually a string) to identify it in the super group.

A cell is always an **interpretation** of some underlying data. For example a series of bytes `7b 22 61 22 3a 31 7d` can be interpreted as a byte array (a single cell with a blob value of `7b 22 61 22 3a 31 7d`) or as an utf-8 encoded string (another cell with a string value of `{"a":1}`) or as a json tree of cells (the root cell being the json object `{}` with a sub cell with label `a` and value `1`). A cell with some value can be always explicitly re-interpreted as another type of cell.

A cell also has a string **type** describing its kind, depending on the interpretation. Such types can be: "file" or "folder" (in the *fs* interpretation), "array" (in the *json* interpretation), "function_item" (in the *rust* interpretation), "response" (in the *http* interpretation), etc.

```ascii
          ┌----------┐
          |   Cell   |
          |----------|
          | [index]  |
          | [label]  |
          | [value]  |
          | [type]   |
          └----------┘
          /         \
         /           \
  ┌-----------┐    ┌------------┐
  | Sub Group |    | Attr Group |
  └-----------┘    └------------┘
     /                   \
    /                     \
  Cell                    Cell
  Cell                    ...
  ...
```

### Examples:

- A *folder* of the file system is a cell. It has a *sub* group and may have *sub* cells (files or folders which it contains); it may also have a *super* cell (parent folder). Its *attr* items are creation/modification date, access rights, size, etc. The folder name is the *label* and has no *value*.

- A *file* of the file system is a cell. It has no *sub* items, may have one *super*, has the same *attr* as a folder and the *label* as its name. A file cell can be *interpreted* in many other ways (string cell, json/yaml/xml cell tree, programming cell trees).

- An entry into a json object is a cell. The parent object is its *super* group. The json key in the key/value pair is the cell *label*. If the value of this json object entry is null or bool or number, then the cell will have a corresponding value and no *sub*; if it's an array or object then the cell will have a *sub* group with the content of the array or object.

- A method in a java project is a cell. It has a parent class (*super*), access attributes (*attr*), and arguments, return type and method body as children (*sub*).

- An http call response is a cell. It has status code and headers as *attr* and the returned body data as its value (a blob). It is usually further interpreted as either a string or json or xml etc.

### Path language

This unified data model naturally supports a path language similar to a file system path, xpath or json path. A cell is always used as a starting point (e.g. the file system current folder). The `/` symbol designates moving to the *sub* group; the `@` symbol to the *attr* group. Jumping to a different interpretation is done using the `^` (elevate) symbol.

As a special case, the starting point of a path is allowed to be a valid url (starting with `http://` or `https://`) or a file system path (must be either absolute, starting with `/`, or relative, starting with `.`).

Other special operators are: the `*` operator which selects any cell in the current group and the `**` operator which selects any cell in current group and any cell descendants in the current interpretation. Filtering these cells is done by boolean expressions in brackets.

Examples:

- `.^file` is the current folder ("." in the `file` interpretation). It is equivalent to just `.`.
- `./src/main.rs` is the `main.rs` file in the ./src/ folder.
- `./src/main.rs@size` is the size of this file (the `size` attribute of the file).

- `./src/main.rs^rust` represents the rust AST tree.
- `./src/main.rs^rust/*[#type=='function_item']` are all the top-level cells representing functions in the `main.rs` rust file.

- `http://api.github.com` is a url cell.
- `http://api.github.com^http` is the http response of a GET request to this url.
- `http://api.github.com^http^json` is the json tree interpretation of a http response of a GET request to this url.
- `http://api.github.com^http^json/rate_limit_url^http^json/resources/core/remaining` makes one http call and uses a field in the respose to make another http call, then select a subfield in the returning json.

- `./src/**^rust` returns a list of all rust files (all files that have a rust interpretation) descending from the `src` folder.
- `./src/**^rust/**/*[#type=="function_item"]` lists all rust functions in all rust files in the `src` folder.
- `./src/**^rust/**/*[#type=="function_item"]/**/*[#type=="let_declaration"]` lists all occurences of *let* declarations in all functions in all rust files in the src folder.
- `./src/**^rust/**/*[#type=="function_item"]/**/*[#type=="let_declaration"][/pattern/*]` lists only destructuring patterns in all occurences of *let* declarations in all functions in all rust files in the src folder. The destructuring patterns are the only ones that have a descendant of `pattern`, for which the filter `[/pattern/*]` is true.

To test the examples yourself, run the `hial` command line tool, e.g.: `hial ls 'http://api.github.com^http^json'`

## What's the current feature implementation status?

See [issues.md](./issues.md).

## What languages are supported?

- Rust API is natively available; Rust is also the implementation language.
- C interop: work in progress.
- Python, Java, Go, Javascript wrappers are planned.

## API examples, use cases

#### - Explore files on the file system

```bash
# shell, works
hial ls "."
```

```rust
// rust works natively
for cell in Cell::from_path(".^file/**").all() {
    // list all file names at the same level
    // indenting children requires more work
    println!("{}: ", cell.read().label());
}
```

<!-- ```python
# python, wip: python interop not done
for cell in hial.path('./**'):
    print(cell.value())
``` -->

#### - Read a list of services from a Docker compose file and print those that have inaccessible images

```bash
# shell, works
echo "Bad images:"
hial ls "./examples/productiondump.json^json/stacks/*/services/*[/image^http@status/code!=200]/name"
```

```rust
// rust, works natively
for service in Cell::from_path("./config.yaml^yaml/services").all() {
    let image = service.to("/image");
    if image.to('^http[@method=HEAD]@status/code') >= 400 {
        println("service {} has an invalid image: {}", service.read().value()?, image.read().value()?);
    }
}
```

<!-- ```python
# python, in progress: python interop not done
for service in hial.search('./config.yaml^yaml/services'):
    image = service.to('/image')
    if image.to('/image^http[@method=HEAD]@status/code') >= 400:
        print(f"service {service.value} has an invalid image: {image}")
``` -->

#### - Change the default mysql port

```bash
# shell, wip
hial "/etc/mysql/my.cnf^ini/mysqld/port = 3307"
```

```rust
// rust, wip
Cell::from_path('/etc/mysql/my.cnf^toml/mysqld/port').write().set_value(3307)?;
```

```python
# python, wip: python interop not done
hial.to('/etc/mysql/my.cnf^toml/mysqld/port').write().set_value(3307)
```
