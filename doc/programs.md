# Hial Program Statements

A Hial program is one or more statements separated by `;` or newlines.

## Valid statement forms

1. Path query (prints matching cells)

```hial
<path_with_start>
```

Examples:

```hial
./src^fs/*
http://api.github.com^http^json/resources/core
$cfg/services/api/image
```

2. Write assignment (writes a scalar value to all matched cells)

```hial
<path_with_start> = <value>
```

Examples:

```hial
./config.yaml^fs[w]^yaml/services/api/replicas = 3
./app.json^fs[w]^json/env/mode = "prod"
$cfg/services/api/image = "my-image:v2"
```

3. Variable binding (binds one variable to the first matched cell)

```hial
$name := <path_with_start>
```

Examples:

```hial
$cfg := ./config.yaml^yaml
$api := :cfg/services/api
$url := http://api.github.com^http^json/rate_limit_url
```

## Values

Valid assignment values are:

- integer literals (example: `42`)
- quoted strings (example: `"hello"` or `'hello'`)
- bare identifiers, treated as strings (example: `dev`)

## Full program example

```hial
$cfg := ./config.yaml^fs[w]^yaml;
$svc := $cfg/services/api;
$svc/image = "my-image:v2";
$svc/replicas = 3;
$svc/image
```
