# Guidelines for writing an interpretation

- The interpretation should make immediate sense to a human reader used with the interpretation domain.
- Cell types are predefined enumerations, are predetermined general classes, not created on runtime. They should be words encountered in the documentation of the interpretation domain (e.g. `array` for a JSON interpretation, or `fn` for a Rust interpretation).
- Types should specify the semantic type (the meaning of the cell in the interpretation context), not the structure of the data (except when the interpretation is low level and the structure is the meaning)
- Labels and values are dynamic data based on the actual content. They should be words encountered in the actual data (e.g. function name as label in a programming language interpretation).
- Labels should not have the same content as values
- A node should have either a value or subs, probably not both
