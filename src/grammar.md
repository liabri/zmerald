# GEMA grammar

This file describes the structure of a GEM file in [EBNF notation][ebnf].

[ebnf]: https://en.wikipedia.org/wiki/Extended_Backus–Naur_form


## Whitespace

```ebnf
ws = { ws_single | comment };
ws_single = "\n" | "\t" | "\r" | " ";
```

## Comments 
```ebnf
comment = ["#", { no_newline }, "\n" | <EOF>];
```

## Commas

```ebnf
comma = ws, ",", ws;
```

## Value

```ebnf
value = unsigned | signed | float | string | char | bool | option | list | map | tuple | struct | enum_variant;
```

## Numbers

```ebnf
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
hex_digit = "A" | "a" | "B" | "b" | "C" | "c" | "D" | "d" | "E" | "e" | "F" | "f";
unsigned = (["0", ("b" | "o")], digit, { digit | '_' } |
             "0x", (digit | hex_digit), { digit | hex_digit | '_' }
           );
signed = ["+" | "-"], unsigned;
float = float_std | float_frac;
float_std = ["+" | "-"], digit, { digit }, ".", {digit}, [float_exp];
float_frac = ".", digit, {digit}, [float_exp];
float_exp = ("e" | "E"), digit, {digit};
```

## String

```ebnf
string = string_marked; # figure out how to allow undelimited strings
string_marked = "\"", { no_double_quotation_marks | string_escape }, "\"";
string_escape = "\\", ("\"" | "\\" | "b" | "f" | "n" | "r" | "t" | ("u", unicode_hex));
```

## Char

```ebnf
char = "'", (no_apostrophe | "\\\\" | "\\'"), "'";
```

## Boolean

```ebnf
bool = "true" | "false";
```

## Optional

```ebnf
option = "None" | option_some;
option_some = "Some", ws, "(", ws, value, ws, ")";
```

## List

```ebnf
list = "[", [value, { comma, value }, [comma]], "]";
```

## Map

```ebnf
map = "{", [map_entry, { comma, map_entry }, [comma]], "}";
map_entry = value, ws, ":", ws, value;
```

## Tuple

```ebnf
tuple = "(", [value, { comma, value }, [comma]], ")";
```

## Struct

```ebnf
struct = unit_struct | tuple_struct | named_struct;
unit_struct = ident;
tuple_struct = [ident], ws, tuple;
named_struct = [ident], ws | id, "{", [named_field, { comma, named_field }, [comma]] | list, "}";
id = ws, "<", value, ">", ws; 
named_field = ident, ws, "=", value;
```

## Enum

```ebnf
enum_variant = enum_variant_unit | enum_variant_tuple | enum_variant_named;
enum_variant_unit = ident;
enum_variant_tuple = ident, ws, tuple;
enum_variant_named = ident, ws, "(", [named_field, { comma, named_field }, [comma]], ")";
```


** this file was inspired by [RONs](https://github.com/ron-rs/ron/blob/HEAD/docs/grammar.md)