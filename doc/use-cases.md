# List of potential use cases

This list of potential use cases should drive the development of the library.

See also the "### What can it do?" section in the README.md file.

### Functions

Special: path(cell path), label, value, serial
General: sort, unique, filter, map, reduce, grouping
Text: concat, regex, len, casing, substrings, split, join, replace, trim, padding, ends_with, starts_with
Aggregation: count, sum, avg, min, max
Math: round, abs
Date: parse, format, add, subtract, diff, duration

```
/question[count(/answer_entities/*)==0]
```

### Python requirements

Update a python module version in a requirements.txt file:

```bash
# change the version of the requests module to 1.2.3
hial './requirements.txt^python.reqs/*[/[0]=="requests"] = "1.2.3"'

# increment the minor version of the requests module
hial 'x = ./requirements.txt^python.requirements/*[/[0]=="requests"]'
hial '$x/[2]^version/:minor += 1'
```
### Move some data from excel to go tests

```
./file.xls^excel/[0]/rows/[1-]/{B,J,A} ->
./file.go^go/'fn Describe("{J}")'/'it("{B}","{A}")'
```

### Search with results structured into a tree

Unclear: what is the accepted language?
```
x = './**/*[.name=='config.yaml'] (as composefile)^yaml/services/*/image[^string^http@status/code!=200]
tree 'result' / [composefile] / image
```

### Transform one format to another

Transform a json file to an xml file and vice versa.

### Structured diff between two files in different formats

```
hial 'diff x y'
```


### - Extract the general structure of a rust file

Get the struct/enum/type definitions (just the name and the type) and the function definitions (just the name and the signature). Get all implementations of traits and the functions inside them, as a tree.

```
hial 'item = ./src/tests/rust.rs^rust/**[:struct_item|:enum_item|:type_item|:function_item]; item/'
```
