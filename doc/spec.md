# Hial Specification

## 1. Overview

### 1.1 Purpose
Hial is a Rust library and CLI for reading, navigating, querying, and in some cases mutating heterogeneous data through a uniform tree abstraction. The product exposes different data sources and formats as cells in a tree and lets users traverse or reinterpret them with a compact path language.

### 1.2 Scope
Included in the current implementation:
- Rust library crate `hiallib`
- CLI program that parses and executes Hial programs
- Uniform cell/group/value API
- Path-based navigation with filtering
- Interpretation/elevation system
- Read and write support for various data sources and formats: path, file system, URL, HTTP, JSON, YAML, TOML, XML, regex matches, split segments, MongoDB, and tree-sitter-backed source trees
- Config and prelude loading from `~/.config/hial/...` files

### 1.3 Intended Users
- Data engineers exploring or transforming structured files
- Devops scripting over config files and HTTP resources
- AI agents exploring and transforming structured data
- programs (via the library API) exploring and transforming structured data

### 1.4 Core Terms
- Cell: a single node in the data tree/graph
- Group: a collection of child cells under `sub`, `attr`, `field`, or elevation relations
- Interpretation: a semantic view over underlying data, selected with `^name`
- Xell: the public runtime wrapper for any cell
- Program: one or more CLI statements executed sequentially

## 2. System Context

### 2.1 External Inputs and Integrations
- Local file system via `path` and `fs`
- HTTP endpoints via `reqwest` blocking client
- MongoDB via the sync MongoDB driver
- Tree-sitter parsers vendored in-repo for Rust and JavaScript; Python and Go sources exist but are not enabled in code
- User config from the home directory

### 2.2 Runtime Context
The CLI loads configuration, optionally executes a prelude program into a shared execution context, parses the user program, and executes statements against live data sources.

### 2.3 High-Level Architecture
Main layers:
1. Parsing layer: `src/prog/parse_program.rs`, `src/prog/parse_path.rs`
2. Execution layer: `src/prog/program.rs`, `src/prog/searcher.rs`
3. Public data API: `src/api/*`
4. Interpretation implementations: `src/interpretations/*`
5. Presentation layer: pretty-printer in `src/pprint/*`

## 3. Functional Requirements

### 3.1 Program Execution
- The CLI must accept a program string from command-line arguments and execute it sequentially.
- A program must support three statement forms:
    - path statement: evaluate a path and print matching cells
    - assignment: evaluate a path and assign a scalar value to each match
    - variable binding: bind the first matching cell to a named variable for later reuse
- Statements must be separable by `;` or by newlines.
- Variable names must support ASCII alphanumeric characters, `_`, and `-`.
- Referencing an undefined variable must return an input error.

### 3.2 Path Language
- A path must start from one of:
    - a file-like string
    - a URL
    - a quoted string literal
    - a variable reference
- Normal traversal must support:
    - `/` for sub-group navigation
    - `@` for attr-group navigation
    - `#` for field-group navigation
- Interpretation changes must use `^interpretation`, with `^` alone meaning auto-detect when available.
- Selectors must support:
    - exact labels
    - `*` for all direct matches in a group
    - `**` for recursive descendant search
    - numeric indexes including negative indexes
- Filters in `[...]` must support:
    - type filters such as `[:function_item]`
    - path truthiness checks such as `[/x]`
    - comparisons against scalar literals such as `[@status/code>=400]`
    - OR-combined expressions with `|`
- Interpretation parameters must support both positional and named syntax, e.g. `^http[HEAD]` and `^fs[w=1]`.
- Automatic interpretation after bare `^` must work where an origin can infer a default interpretation, such as file system cells inferring JSON from `.json`.


### 3.3 Data Access Model
- Every resolved node must be exposed as an `Xell`.
- An `Xell` must expose:
    - `read()` for type/label/value/index/serialization access
    - `write()` for mutation where supported
    - `sub()`, `attr()`, and `field()` navigation where supported
    - `be()` or equivalent elevation into alternate interpretations
    - `to()` and `all()` path evaluation helpers
- Values must support null, boolean, integer, float, string, and byte-oriented representations.

### 3.5 Mutation and Persistence
- Assignment statements must call `write().value(...)` on every matched cell.
- The system must support write policies:
    - `ReadOnly`
    - `NoAutoWrite`
    - `WriteBackOnDrop`
- Mutable structured interpretations must be able to serialize back to their origin data when `save()` or automatic writeback is used.

### 3.6 CLI Output
- Path statements must pretty-print matching cells.
- CLI options must support:
    - `-v` or `--verbose`
    - `-d <depth>`
    - `-b <breadth>`
    - `--no-color`
    - `--color <dark|light|none>`
    - `--` to terminate flag parsing

### 3.7 Configuration
- Main configuration must load from `~/.config/hial/hial.yaml` when present.
- Prelude must load from `~/.config/hial/prelude.hial` when present and execute before the user program.
- Missing config or prelude files must not be treated as fatal errors.

## 4. Non-Functional Requirements

### 4.1 Reliability
- Parse errors must include location-aware diagnostics.
- Searches should degrade gracefully when optional elevations are unavailable after wildcard traversal.
- Nonexistent data should usually surface as `HErrKind::None` rather than panicking.

### 4.2 Performance
- Path search uses a stack-based DFS matcher rather than recursive function calls.
- The implementation is designed for interactive CLI/library use, not for high-throughput concurrent serving.
- HTTP is synchronous and blocking.

### 4.3 Safety
- The system must avoid unsafe persistence outside explicit write policies.
- Tree-sitter integration uses an unsafe lifetime transmute internally; this is an implementation risk and should be treated as a maintenance hotspot.

### 4.4 Portability
- The project targets Rust edition 2024.
- Config path resolution currently assumes a home-directory-based user environment.

### 4.5 Observability
- Verbose logging can be enabled from the CLI.
- Errors use the internal `HErr` hierarchy with kinds such as input, IO, invalid format, internal, and none/no-result cases.

## 5. Data Model

### 5.1 Core Entity: Cell
A cell is the universal unit of data. Each cell may expose:
- `type`
- `index`
- `label`
- `value`
- parent/head relation
- `sub` children
- `attr` children
- `field` children

### 5.2 Groups
Groups are typed collections of cells with:
- label model metadata (`LabelType`)
- indexed access
- label-based lookup
- optional create/add support

### 5.3 Domain and Origin
Cells are wrapped in a domain that tracks:
- write policy
- origin cell
- dirty state
- root dynamic cell cache

This allows edits made in derived interpretations to be saved back to their origin.

### 5.4 Value Types
Primitive value support includes:
- `None`
- `Bool`
- `Int`
- `Float`
- `Str`
- `Bytes`

## 6. API Specification

### 6.1 Rust API
Primary entry point:
- `Xell`

Key operations:
- `Xell::new(path_like)`
- `Xell::from(value)`
- `be("interpretation")`
- `to("path")`
- `all("path")`
- `read()`
- `write()`
- `save(...)`
- `origin()`
- `policy(...)`

Programmatic execution:
- `Program::parse(&str)`
- `Program::run(...)`
- `Program::run_in_context(...)`

### 6.2 CLI Interface
Observed current behavior:
- binary entrypoint is `src/main.rs`
- package/binary naming currently resolves to `hiallib` in Cargo-based test execution
- README examples still refer to the command as `hial`

The naming mismatch should be resolved or documented explicitly in user-facing packaging.

### 6.3 Error Handling
- API methods return `Res<T>`
- recoverable absence commonly uses `HErrKind::None`
- malformed input uses `HErrKind::Input`
- storage/network/serialization failures use IO or format-related error kinds

## 7. Architecture

### 7.1 Interpretation Registration
Interpretations are registered through distributed slices of elevation constructors. A source interpretation can advertise one or more target interpretations, and elevation resolves through that registry at runtime.

### 7.2 Search Engine
The search engine:
1. parses a path into path items
2. walks candidate cells with an explicit stack
3. evaluates normal path items against groups
4. evaluates elevation items through interpretation lookup
5. applies filters as nested path evaluations/comparisons
6. yields matched cells lazily

### 7.3 Persistence Model
Writable interpretations mutate an in-memory domain representation. Persistence occurs either:
- explicitly through `save()` or `save_domain()`
- implicitly when write policy is `WriteBackOnDrop`

### 7.4 Tree-Sitter Parsing
Source-code interpretations convert source text into a tree-sitter tree and surface named nodes as cells. Rust support is covered by tests. JavaScript parser assets are present and wired. Python and Go assets are vendored but not enabled in the parser switch.

## 8. Current Constraints and Gaps

### 8.1 Implemented vs Documented
The README contains aspirational features that are not implemented in the current code, including copy, diff, markdown, zip, git, and generic tree conversion flows.

### 8.2 Partial Language Support
- Rust source interpretation is tested for navigation.
- JavaScript appears wired but is not covered by tests in this repository.
- Python and Go cannot currently be instantiated successfully despite being listed as targets.

### 8.3 Incomplete Interop
- `c_api.rs` exists, but the module is not exported in `src/lib.rs`.
- `hial.h` and `cbindgen.toml` exist, indicating intended C interop, but this is not a stable delivered feature in the current crate surface.

### 8.4 Network Dependence
HTTP and some search tests depend on external connectivity and are skipped when the network is unavailable.

## 9. Acceptance Criteria For The Current Product

The product can be considered compliant with this spec if:
1. A user can query local structured files and HTTP responses with the path language.
2. A user can assign scalar values through the CLI to writable targets.
3. A Rust caller can navigate and mutate supported interpretations via `Xell`.
4. Path parsing supports wildcard, recursive wildcard, indexes, filters, and elevations.
5. Config and prelude loading work from the documented config directory.
6. Unsupported roadmap features fail clearly rather than appearing to work.

## 10. Recommended Next Spec Revisions

- Separate "current implementation" from "roadmap" in README and docs
- Formalize the path grammar in EBNF
- Define supported write semantics per interpretation
- Decide the canonical CLI binary name
- Either enable or remove incomplete language targets from the public interpretation list
