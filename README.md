
# Hial

Hial is an uniform data API, particularly suitable for textual data. It is both a library and an executable.

🚧 Hial is currently under construction. 🚧

## What is an uniform data API?

An uniform data API is a programmatic interface to different types of data represented in an uniform manner (a general tree/graph structure). This makes the data easy to read, explore and modify using a small number of functions. The types of data that can be supported by this API are the file system structure, usual configuration files (json, yaml, toml), markup files (xml, html), programs written in various programming languages, operating system configurations and runtime parameters, etc.

### Why is it needed?

The uniform data API maximizes developer comfort and speed. A simple and uniform data model makes it easy to create, transform, filter, delete any kind of data, programmatically. Automated programmatic editing, custom verification of programs, semantic diffs and similar tasks become possible.

### Should you use it?

For devops, this provides programmatic access to configuration files. Programmers can use it as a structured grep utility, for complex data conversions, for testing program invariants, etc.

One should probably not use it for performance intensive tasks or large amounts of data. But it should be used for prototyping such cases.

**Please be warned** that the current implementations are neither performant nor failproof. The project is currently in alpha state and the APIs will most likely suffer extensive changes.

## What's the data model?

The data model is that of a graph (usually just a tree) of simple data nodes.

Each data node is called a **cell**, and each cell is linked to other cells. The linked cells are organized into **groups**. Childrens of a cell are in the **sub** ("subordinate") group, parent cell(s) are the **super** ("superordinate") group, and cell attributes or properties are put into the **attr** group.

Each cell may have a numeric **index** and/or a **label** that makes it reachable within the group, and may have a pure data **value**. A value can be: null, bool, number, string or blob (byte array).

A cell is an **interpretation** of some underlying data. For example a series of bytes can be interpreted as a byte array (a cell with a blob value) or as an utf-8 encoded string (another cell with a string value) or as a json tree (a tree of cells). From a cell of a certain interpretation we can jump to the cell of a different interpretation.

A cell also has a string **type**describing its kind, depending on the interpretation. Such types can be: "file", "folder", "array" (in the json interpretation), "function_item" (in the rust interpretation), "response" (in the http interpretation), etc.

### Examples:

- A *folder* of the file system is a cell. It may have *sub* items (files or folders which it contains) and may have a *super* (parent folder). Its *attr* items are creation/modification date, access rights, size, etc. The folder name is both the *label* and the *value*.

- A *file* of the file system is a cell. It has no *sub* items, may have one *super*, has the same *attr* as a folder and both a *label* and *value* as its name. A file cell can be *interpreted* in many other ways (string cell, json/yaml/xml cell tree, programming cell trees).

- An entry into a json object is a cell. The parent object is its *super*. The json key in the key/value pair is the cell *label*. If the value of this json object entry is null or bool or number, then the cell value will be the same; if it's an array or object then the cell value will be null and the cell will have a *sub* group with the content of the array or object.

- A method in a java project is a cell. It has a parent class (*super*), access attributes (*attr*), and arguments, return type and method body as children (*sub*).

- An http call response is a cell. It has status code and headers as *attr* and the returned body data as its value (a blob). It is usually further interpreted as either a string or json or xml etc.

### Path language

This unified data model naturally supports a path language similar to a file system path, xpath or json path. A cell is always used as a starting point (e.g. the file system current folder). The '/' symbol designates moving to the *sub* group; the '@' symbol to the *attr* group. Jumping to a different interpretation is done using the '^' (elevate) symbol.

As a special case, the starting point of a path is allowed to be a valid url (starting with http:// or https://) or a file system path (must be either absolute, starting with '/', or relative, starting with '.').

Other special operators are: the '\*' operator which selects any cell in the current group and the '\*\*' operator which selects any cell in current group and any cell descendants in the current interpretation; filtering these cells is done by boolean expressions in brackets.

Examples:

- `.^file` is the current folder ("." in the `file` interpretation). It is equivalent to just `.`.
- `./src/main.rs` is the `main.rs` file in the ./src/ folder.
- `./src/main.rs@size` is the size of this file (the `size` attribute of the file).

- `./src/main.rs^rust` represents the rust AST tree.
- `./src/main.rs^rust/*[#type=='function_item']` are all the top-level cells representing functions in the `main.rs` rust file.

- `http://api.github.com` is a url cell.
- `http://api.github.com^http` is the http response of a GET request to this url.
- `http://api.github.com^http^json` is the json tree interpretation of a http response of a GET request to this url.
- `http://api.github.com^http^json/rate_limit_url#value^http^json/resources/core/remaining` makes one http call and uses a field in the respose to make another http call, then select a subfield in the returning json.

- `./src/**^rust` returns a list of all rust files (all files that have a rust interpretation) descending from the `src` folder.
- `./src/**^rust/**/*[#type=="function_item"]` lists all rust functions in all rust files in the `src` folder.
- `./src/**^rust/**/*[#type=="function_item"]/**/*[#type=="let_declaration"]` lists all occurences of *let* declarations in all functions in all rust files in the src folder.
- `./src/**^rust/**/*[#type=="function_item"]/**/*[#type=="let_declaration"][/pattern/*]` lists only destructuring patterns in all occurences of *let* declarations in all functions in all rust files in the src folder. The destructuring patterns are the only ones that have a descendant of `pattern`, for which this filter: `[/pattern/*]` is true.

To test the examples yourself, run the `hial` tool in bash, e.g.: `hial explore 'http://api.github.com^http^json'`

## What's the current feature implementation status?

See [issues.md](./issues.md).

## What languages are supported?

- [x] Rust API is natively available; Rust is also the implementation language.
- [ ] C interop: work in progress.
- [ ] Python, Java, Go, Javascript wrappers are planned.

## API examples, use cases

#### Explore files on the file system

```bash
# bash, works
hial explore "."
```

```python
# python, in progress: python interop not done
for cell in path('./**'):
    print(cell.value())
```

#### Read a list of services from a Docker compose file and print those that have inaccessible images [working]

```bash
# bash, works
echo "Bad images:"
hial print "./config.yaml^yaml/services[/image#value^http@status/code!=200]/name"
```

```python
# python, in progress: python interop not done
for service in cell('config.yaml').be('file').be('yaml').sub('services'):
    name = service.value
    image = service.sub('image')
    if image.be('http').attr('status').sub('code') >= 400:
        print(f"service {name} has an invalid image: {image}")
```
