# jsfx-lint

jsfx-lint is a static analyzer and linter for [JSFX](https://www.cockos.com/reaper/sdk/js/js.php), REAPER's amazing
language to create audio and MIDI plugins. This linter is written in Rust
and uses the [grmtools](https://github.com/softdevteam/grmtools) library to generate a parser for EEL2.
[WDL](https://www.cockos.com/wdl/) is used to handle preprocessing.

- [jsfx-lint](#jsfx-lint)
- [Features](#features)
- [Motivation](#motivation)
- [Installation](#installation)
  - [Pre-built binaries (recommended)](#pre-built-binaries-recommended)
  - [From source](#from-source)
- [Usage](#usage)
  - [CLI](#cli)
  - [Config](#config)
- [Development](#development)
  - [The program struct](#the-program-struct)
  - [Process overview](#process-overview)
  - [Creating a lint](#creating-a-lint)
- [Lints](#lints)
  - [arg\_must\_be\_namespace](#arg_must_be_namespace)
  - [arg\_never\_read](#arg_never_read)
  - [discarded\_param](#discarded_param)
  - [duplicate\_argument](#duplicate_argument)
  - [duplicate\_modifier](#duplicate_modifier)
  - [empty\_modifier](#empty_modifier)
  - [fully\_shadowed\_argument](#fully_shadowed_argument)
  - [global\_never\_read](#global_never_read)
  - [global\_never\_written](#global_never_written)
  - [implicit\_parens](#implicit_parens)
  - [implicitly\_passing\_zero](#implicitly_passing_zero)
  - [import\_not\_found](#import_not_found)
  - [inaccessible\_global](#inaccessible_global)
  - [incompatible\_contexts](#incompatible_contexts)
  - [inconsistent\_casing](#inconsistent_casing)
  - [invalid\_lhs\_assignment](#invalid_lhs_assignment)
  - [looks\_like\_slider\_n](#looks_like_slider_n)
  - [loop\_as\_r\_value](#loop_as_r_value)
  - [object\_access\_unused](#object_access_unused)
  - [overwriting\_arg](#overwriting_arg)
  - [param\_count](#param_count)
  - [parse\_error](#parse_error)
  - [partially\_shadowed\_argument](#partially_shadowed_argument)
  - [read\_unreadable](#read_unreadable)
  - [ref\_arg\_in\_incompatible\_modifier](#ref_arg_in_incompatible_modifier)
  - [sample\_section\_without\_audio\_io](#sample_section_without_audio_io)
  - [shadowed\_slider\_n](#shadowed_slider_n)
  - [slider\_invalid\_identifier](#slider_invalid_identifier)
  - [slider\_labels](#slider_labels)
  - [slider\_out\_of\_range](#slider_out_of_range)
  - [slider\_over\_max\_id](#slider_over_max_id)
  - [slider\_parser](#slider_parser)
  - [slider\_without\_description](#slider_without_description)
  - [sprintf\_params](#sprintf_params)
  - [string\_arg\_in\_local\_mod](#string_arg_in_local_mod)
  - [too\_many\_params](#too_many_params)
  - [unknown\_function](#unknown_function)
  - [unknown\_section](#unknown_section)
  - [unknown\_slider](#unknown_slider)
  - [unnecessary\_comma](#unnecessary_comma)
  - [unnecessary\_semicolon](#unnecessary_semicolon)
  - [unreachable\_function](#unreachable_function)
  - [unused\_function](#unused_function)
  - [unused\_modifier\_arg](#unused_modifier_arg)
  - [unused\_slider](#unused_slider)
  - [useless\_expression](#useless_expression)
  - [useless\_object](#useless_object)
  - [useless\_ref\_arg](#useless_ref_arg)
  - [write\_to\_non\_writable](#write_to_non_writable)
  - [wrong\_context](#wrong_context)
  - [wrong\_section\_param](#wrong_section_param)
- [Known issues, limitations and TODOs](#known-issues-limitations-and-todos)
- [License](#license)

# Features

- Supports EEL2 preprocessor blocks (`<? ?>`, `_suppress`, etc.)
- Parses meta information (sliders, options, etc.)
- Provides various style and correctness [lints](#lints)
- Reports syntax errors

# Motivation

JSFX, the language used in REAPER for creating audio plugins, is a fantastic learning tool and a great prototyping
sandbox. However, its design can be quirky and prone to errors. This project aims to provide a linter that helps catch
these errors, allowing you to spend less time scratching your head and more time experimenting.

jsfx-lint aspires to be a comprehensive linter for JSFX, addressing a wide range of issues from simple style problems to
more complex and less obvious issues. The goal is to align closely with the official JSFX implementation.

At the moment, jsfx-lint is a CLI tool. However, work on an LSP server for integration with text editors is planned.

# Installation

## Pre-built binaries (recommended)

Pre-built binaries are available on the [releases](https://github.com/Souk21/jsfx-lint/releases) page.

Please note that the `eel_pp` binary must be located in the same directory as the `jsfx-lint` binary for the program to
function correctly.

## From source

Prerequisites: Rust needs to be installed.

```sh
git clone https://github.com/Souk21/jsfx-lint.git
cd jsfx-lint
cargo build --release
```

`eel_pp` binaries can be
built [from the WDL/EEL2 repo](https://github.com/justinfrankel/WDL/blob/main/WDL/eel2/eel_pp.cpp).

During the build process, the appropriate `eel_pp` binary from the `eel_pp/` directory is automatically copied to
the `target/debug` or `target/release` directory. This ensures that the `eel_pp` binary is in the correct location
for `jsfx-lint` to operate and for GitHub Actions to package the binaries correctly.

# Usage

## CLI

```sh
jsfx-lint [filepath | -]
```

`filepath` is the path to the JSFX file to lint. If `-` is provided, the program reads from `stdin`.

Example usage:
```sh
# From file
jsfx-lint path_to/effect.jsfx

# From stdin
echo "@init\na = 1;" | ./jsfx-lint -
```

## Config

To customize the linter, add a `config.toml` file in the same directory as the `jsfx-lint` binary. This file lets you override the default severity for specific lints. For example:

```toml
arg_never_read = "silent"
# ...
```

Any lints not listed in the config file will use their default severity. Valid lint names can be found in the [Lints](#lints) section of this README.

# Development

## The program struct

In JSFX, a program is composed of multiple sections, each containing a chunk of EEL2 code. The sections must be of one
of these types: `@init`, `@block`, `@sample`, `@gfx`, `@serialize` or `@slider`.
A file can import other files using the `import` directive. Quoting the JSFX docs:
> Importing files via this directive will have any functions defined in their @init sections available to the local
> effect. Additionally, if the imported file implements other sections (such as @sample, etc), and the importing file
> does
> not implement those sections, the imported version of those sections will be used.

One thing this doesn't mention is that not only the function definitions in @init are imported, the code is also
imported. It's as if the code of the imported section was pasted in the importing file, before its first @init section.

If a file contains multiple `@section` of
the same type, they are merged into a single section, in order of their definition in the source file.

In `jsfx-lint`, the `Program` struct is the main data structure representing a JSFX program. It contains the parsed AST,
metadata, and symbols.

It is structured as follows:

- **Metadata**
    - Metadata includes sliders and their parameters (min, max, default, etc.), options, imports, pins, etc
- **Sections:**
    - **Chunks**
        - Corresponds to a single @section block in one of the source files.
    - **Function definitions**
        - Arguments
        - Scope
    - **Function calls**
        - Prefix
        - Parameters

## Process overview

The linter operates in three passes:

1. **Parsing and Metadata Collection:** The entry file source is first preprocessed by `eel_pp` if necessary. Each `@section` EEL2 source is then parsed, and
   metadata (everything that's not EEL2: `sliders`, `import`, etc.) is collected. At this stage, imports are resolved recursively: every imported file also undergoes optional preprocessing and EEL2/meta parsing. This process continues until all imports are handled. The result of this pass is a low-level structure containing the
   AST nodes of each sections chunk (a section can be defined multiple times!) along with the collected metadata.
2. **Symbol Collection:** During this pass, the AST is traversed to collect symbols, which include functions, variables,
   scopes, and more. The outcome of this pass is a high-level representation of the program, complete with the symbols
   and their respective scopes.
3. **Linting:** In the final pass, lints are run using both the low-level and high-level representations of the program.

## Creating a lint

The first step of creating a new lint is to add an entry to the `data/config.default.toml` file. The key should be the
lint name (in snake_case), and the value should be the default severity of the lint. The severity can be one of the
following: `error`, `warning`, `style`, or `silent`.

```toml
# ...
"my_lint" = "warning"
# ...
```

> [!TIP]
> - Use `error` only for lints that trigger an error with the official JSFX implementation.
> - Use `warning` for lints that are likely to be mistakes or lead to confusing code.
> - Use `style` for more opinionated or cosmetic lints.

The lint will then automatically be added to the `IssueKind` enum as a PascalCase variant (e.g. `IssueKind::MyLint`).
This process takes place in the [`build.rs`](src/build.rs) file.

A new module needs to be added to the [`src/lints/`](src/lints) directory. Usually, the module has the same name as the config entry. (e.g. `my_lint.rs` for the `my_lint` config entry).

The module should contain a public function of the type:

```rust
type Lint = fn(&Program, &mut Issues);
```

This function's role is to add the issues found in the program to `Issues`, using the `IssueKind` generated above.
The `Program` struct contains the parsed AST and other information about the file.

```rust
pub fn lint(program: &Program, issues: &mut Issues) {
    // ...
    issues.add(IssueKind::MyLint, ...);
}
```

It is recommended to create tests for each lint. The tests should be in the same file as the lint, inside a `mod tests` module.

```rust
#[cfg(test)]
mod tests {
    // `indoc!` removes indentation from multiline string
    use indoc::indoc;
    use crate::file::File;
    use crate::IssueKind;

    #[test]
    pub fn test_name() {
        // Minimal JSFX source code that triggers `IssueKind::MyLint`
        let source = indoc! {"
            @init
            function foo() (
                0;
            );"
        };
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::MyLint));
    }
}
```

The module then needs to be imported into [`lints/mod.rs`](src/lints/mod.rs) as follows:

```rust
// ...
mod my_lint;
// ...

pub fn get() -> Vec<Lint> {
    vec![
        // ...
        my_lint::lint,
        // ...
    ]
}
```

Lastly, you need to document the lint in the `README.md` file, in the [Lints](#lints) section.

# Lints

## arg_must_be_namespace

Default severity: `error`

Argument must be a namespace (`ref`).

```
function foo(ref*) ( ref.bar = 1; );
foo(0); // 0 is not a namespace
```

## arg_never_read

Default severity: `warning`

Argument is never read.

```
function foo(bar) ( 0 );
```

## discarded_param

Default severity: `warning`

A function with no arguments can also be called with one argument, which ends up being discarded.

```
function foo() (0);
foo(1); // 1 is discarded
```

## duplicate_argument

Default severity: `warning`

Argument appears multiple times in function definition.

```
function foo(bar, bar) (0);
```

```
function foo() local(bar) local(bar) (0);
```

## duplicate_modifier

Default severity: `style`

A modifier is defined multiple times.

```
function foo() local(a) local(b) (0);
```

## empty_modifier

Default severity: `silent`

Empty modifier (other than `global`) in a function definition.

```
function foo() local() (0);
```

## fully_shadowed_argument

Default severity: `warning`

Modifier argument is fully shadowed by a function argument.
Note: Function arguments that are named `this` or that start with `this.` are not reachable as they are shadowed by the `this` keyword.

```
function foo(a) local(a) (0);
```

## global_never_read

Default severity: `warning`

A variable is never read. Note that a variable defined as `_` will be ignored by this lint.


## global_never_written

Default severity: `warning`

A variable is never written. Note that a variable defined as `_` will be ignored by this lint.

## implicit_parens

Default severity: `style`

Function definition with implicit parenthesis.

```
function foo local() (
  0
);
```

## implicitly_passing_zero

Default severity: `warning`

Functions with one argument can be called with zero parameters, and the argument end up being 0.

```
function foo(bar) (0);
foo(); // Implicitly calls foo(0)
```

## import_not_found

Default severity: `error`

The import was not found.

```
import non_existing.jsfx-inc
```

## inaccessible_global

Default severity: `error`

Accessing a variable from a function with a `global` modifier that is empty or that doesn't contain said variable.

```
function foo() global() local(a) (
  a = beat_position;
);
```

```
function foo() global(a) (
  a = beat_position;
);
```

## incompatible_contexts

Default severity: `warning`

Function uses variables or calls functions with incompatible contexts.

```
function foo() (
  spl0 = 1; // only valid in @sample
  gfx_rect(0, 0, 100, 100); // only valid in @gfx
);
```


## inconsistent_casing

Default severity: `style`
A variable or function called with different casing throughout the file.

```
function foo() (0);
FOO();
```

```
bar = 1;
baz = BAR;
```


## invalid_lhs_assignment

Default severity: `warning`

The left-hand side of the assignment is not assignable.

```
(1 + 1) = 1;
floor(1) = 1;
```

## looks_like_slider_n

Default severity: `warning`

A `sliderN` variable with `N` greater than 256 or `N` equal to 0.

```
a = slider0;
a = slider257;
```

## loop_as_r_value

Default severity: `warning`

A loop used as the right-hand side of an assignment.

```
a = while(i < 10) (
  i += 1;
  i
);
// While always return 0, Loops always return 1.
```

## object_access_unused

Default severity: `warning`

A value accessed on a `instance`/`this`/`ref` is not accessed anywhere else.

```
function foo() ( this.bar = 1; );
object.foo(); // Sets object.bar to 1
// ... code not using object.bar
```

```
function foo(ref*) ( ref.bar = 1; );
foo(object); // Sets object.bar to 1
// ... code not using object.bar
```

```
function foo() instance(bar) ( bar = 1; );
object.foo(); // Sets object.bar to 1
// ... code not using object.bar
```

## overwriting_arg

Default severity: `warning`

An argument is assigned before being read.

```
function foo(a) (
  a = 1;
);
```

## param_count

Default severity: `error`

A function is called with too many or too few parameters.

```
foo = abs(1, 2);
gfx_rect(1);
```

## parse_error

Default severity: `error`

The parser encountered an error or unexpected token. Example:

```
a (= 1;
```

## partially_shadowed_argument

Default severity: `warning`

An argument present both in the arg list and in a modifier is partially shadowed.

```
function foo(a) instance(a) (
  _ = a; // Here the argument is used
  _ = a.bar; // Here the instance is used
  0
);
```

## read_unreadable

Default severity: `warning`

## ref_arg_in_incompatible_modifier

Default severity: `error`

Ref args (`arg*`) are only allowed in a `global` or `globals` modifier.

```
function foo() local(bar*) (0);
```

## sample_section_without_audio_io

Default severity: `warning`

A @sample section without audio input/output. The code should be moved to the @block section.

```
@sample
gain = slider1;
```

## shadowed_slider_n

Default severity: `warning`

Sliders that are bound to a variable can't be accessed using sliderN variables.

```
slider1:foo=0<0,1,1>Foo
@init
slider1 = 2; // Will not set slider1 to 2
```

## slider_invalid_identifier

Default severity: `warning`

Slider identifier is not a valid EEL2 identifier. It must start with an alphanumeric character or '_', and can only contain letters, numbers, commas and underscores.

```
slider1:gain-db=0<0,1,1>Gain dB
```

## slider_labels

Default severity: `warning`

Sliders with labels should have their minimum equal to 0, their step equal to 1, and their maximum equal to the number of
labels.

```
slider1:0<0,3,1{One,Two}>Foo // Maximum should be 2
```

## slider_out_of_range

Default severity: `warning`

A slider is set to a value above its maximum or below its minimum.

```
slider1:0<0,1,1>Foo
@init
slider1 = -1; // Below minimum
slider1 = 2; // Above maximum
```

## slider_over_max_id

Default severity: `error`

A slider is defined with an id greater than 256.

```
slider257:1<0,1,1>foo
```

## slider_parser

Default severity: `warning`

The slider parser encountered an error. This can happen if the slider definition is malformed.

```
slider1:0<0,text,1>Foo
slider1:0<0,1,1{One,Two>Foo
```

## slider_without_description

Default severity: `warning`

A slider without a description. It will not be displayed in the default UI and will not be automatable.

```
slider1:0<0,1,1>
```


## sprintf_params

Default severity: `error`

`sprintf` is called with the incorrect number of arguments.

```
sprintf(#str, "%d %d", 1);
```

## string_arg_in_local_mod

Default severity: `error`

`#strings` are not allowed in the `local()` modifier.

```
function foo() local(#str) (0);
```

## too_many_params

Default severity: `error`

A function is defined with too many parameters. The maximum is 40.

```
function foo(p1, p2, ... p42, p43) (0);
```

## unknown_function

Default severity: `error`

The called function doesn't exist.

## unknown_section

Default severity: `error`

`@section` other than `@init`,  `@sample`, `@serialize`, `@block`, `@gfx`, or `@slider`.

## unknown_slider

Default severity: `error`

Accessing a slider that is not defined.

## unnecessary_comma

Default severity: `style`

Unnecessary comma(s) in function/modifier argument list. Note: trailing commas are allowed.

```
function foo(bar,, baz) (0);
```

## unnecessary_semicolon

Default severity: `style`

Unnecessary semicolon(s) in a function body.

```
@init
a;;
b;
```

## unreachable_function

Default severity: `error`

A function is unreachable because it is shadowed by the `loop` or `while` keywords.

```
function loop(foo, bar) (0);
```

```
function while() (0);
```

```
function while(foo) (0);
```

## unused_function

Default severity: `warning`

A function is never called.

## unused_modifier_arg

Default severity: `warning`

A modifier argument is never used.

```
function foo() local(unused) ( 0 );
```

```
function foo() global(unused) ( 0 );
```

## unused_slider

Default severity: `warning`

A slider is never read or written to.

## useless_expression

Default severity: `warning`

An expression doesn't have any side effects.

```
@init
function foo() (
  1 + 1; // This expression does nothing
  0;
);
```

## useless_object

Default severity: `warning`

A function does not need to be called on an pseudo-object.

```
function foo() ( 0; );
bar.foo();
```

## useless_ref_arg

Default severity: `warning`

An argument does not need to be `ref`.

```
function foo(a*) ( b = a; );
```

## write_to_non_writable

Default severity: `warning`

Writing to a non-writable variable.

```
beat_position = 10;
```

## wrong_context

Default severity: `warning`

A variable or function is used in the wrong context.

```
@init
spl0 = 1; // spl0 can only be used in @block or @sample
```

## wrong_section_param

Default severity: `error`

`@section` (except `@gfx`) with parameters. `@gfx` with more than 2 parameters (or only 1).`@gfx` with non-integer params.

```
@gfx 10 20 30
```

```
@init 10 20
```

```
@gfx 10 a+b
```

# Known issues, limitations and TODOs
- When a file contains multiple `@section` of the same type, JSFX combines them into a single section by concatenating their text. This means the following is allowed:

  ```eel
  @init
  a =
  @block
  b = 2;
  @init
  1;
  ```
  and is equivalent to:
  ```eel
  @init
  a = 1;
  @block
  b = 2;
  ```
  This behavior of joining section text is not yet implemented in `jsfx-lint`,  which currently keeps multiple `@section` blocks of the same type separate. Work is planned to address this in the future.
- [JSFX looks in the `REAPER/Effects` directory when resolving imports.](https://askjf.com/index.php?q=7154s) Right now, `jsfx-lint` only knows about the default `Effects` path (`~/.config/REAPER/Effects` on Linux, `%APPDATA%\REAPER\Effects` on Windows, and `~/Library/Application Support/REAPER/Effects` on macOS). It is currently not possible to specify additional or alternative paths for imports. This is planned for a future release.
- Currently, `jsfx-lint` does not show style or warning lints that are located in the output of a preprocessor block (i.e. `<? ?>`). This is to avoid flooding the output with lints that are not relevant to the user. This should be a flag/config option in the future.
- There are still many simple optimizations that can be done to the linter.
- Lint ideas:
  - Warn about unknown `option`, or incorrect parameters for `option`
  - Track shadowed state for functions to be able to report when a function is unused because it is shadowed
  - Warn when a variable does not need to be an `instance`, and could be a `local`
  - `slider()` function lints
  - If an argument is overwritten before being read, it might be a `local()`

# License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
This project uses the EEL2 library, which is BSD-licensed. For more details, please refer to the BSD license text
included in the [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES) file.
