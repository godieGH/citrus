# Citrus Language Reference

> Citrus is a statically typed, compiled language focused on clarity, safety, and simplicity.
> Ownership and borrowing follow Rust semantics. No garbage collector. No null by default.
> No classes, no inheritance — structs, traits, and implementations only.

---

## Table of Contents

1. [Naming Conventions](#naming-conventions)
2. [Comments](#comments)
3. [Variables](#variables)
4. [Types](#types)
5. [References and Borrowing](#references-and-borrowing)
6. [Operators](#operators)
7. [Functions](#functions)
8. [Anonymous Functions](#anonymous-functions)
9. [Structs](#structs)
10. [Enums](#enums)
11. [Traits](#traits)
12. [Implement](#implement)
13. [Generics](#generics)
14. [Option Type](#option-type)
15. [Error Handling](#error-handling)
16. [Control Flow](#control-flow)
17. [Iterators and Ranges](#iterators-and-ranges)
18. [Arrays and Vectors](#arrays-and-vectors)
19. [Macros](#macros)
20. [Attributes](#attributes)
21. [Modules and Imports](#modules-and-imports)
22. [Visibility](#visibility)
23. [Ownership Rules Summary](#ownership-rules-summary)

---

## Naming Conventions

| Thing                        | Convention   | Example               |
|------------------------------|--------------|-----------------------|
| Variables                    | snake_case   | `player_score`        |
| Functions                    | snake_case   | `calculate_total`     |
| Constants (static)           | UPPER_SNAKE  | `MAX_SIZE`            |
| Structs                      | PascalCase   | `PlayerData`          |
| Traits                       | PascalCase   | `Drawable`            |
| Modules / Files              | snake_case   | `math_utils`          |
| Generic type parameters      | PascalCase   | `T`, `TValue`         |
| Macro names                  | snake_case   | `println!`, `assert!` |

---

## Comments

```
// single line comment

# also a single line comment
```

No multi-line comments. Use consecutive `//` lines for blocks.

---

## Variables

### Syntax
```
let name as Type = value;
```

### Forms

```citrus
// typed and initialized
let score as Int_32 = 100;

// type inferred — built-in types only
let score = 100;

// mutable
let mutable score as Int_32 = 0;

// uninitialized — must provide type, no inference allowed
let score as Int_32;

// constant — immutable, global or local, never reassigned
static MAX_SCORE as UInt_32 = 9999;
```

### Rules
- All variables are **immutable by default**
- `mutable` must be explicit
- Uninitialized variables must have a type annotation
- Type inference only works for built-in primitive types
- Custom types, generics — always annotate explicitly

---

## Types

### Text

```citrus
let msg as Text = "Hello World";
```

- Heap allocated
- Owned — not copied, **moved** unless explicitly cloned
- Immutable by default
- No stack string type

**String literal forms**

```citrus
"Hello World"             // regular — escape sequences apply  \" \\ \n \t
R"no escapes here"        // raw — backslash is literal
R#"can "quote" freely"#   // raw hash — can contain " inside
```

### Char

```citrus
let c as Char = 'F';
let newline as Char = '\n';
let tab     as Char = '\t';
```

Single character. Uses single quotes. Supports escape sequences.

### Bool

```citrus
let active as Bool = true;
let done   as Bool = false;
```

### Integers

| Type    | Size    | Range                          |
|---------|---------|--------------------------------|
| Int_8   | 8-bit   | -128 to 127                    |
| Int_32  | 32-bit  | -2,147,483,648 to 2,147,483,647|
| Int_64  | 64-bit  | large signed                   |
| Int_128 | 128-bit | very large signed              |
| UInt_8  | 8-bit   | 0 to 255                       |
| UInt_32 | 32-bit  | 0 to 4,294,967,295             |
| UInt_64 | 64-bit  | large unsigned                 |
| UInt_128| 128-bit | very large unsigned            |

### Floats

| Type     | Size    |
|----------|---------|
| Float_32 | 32-bit  |
| Float_64 | 64-bit  |

### Numeric Literal Forms

```citrus
let a as Int_32   = 60;
let b as Int_32   = 1_000_000;    // underscore separator
let c as Int_32   = 0xFF;         // hexadecimal
let d as Int_32   = 0b1010_1010;  // binary
let e as Int_32   = 0o77;         // octal
let f as Float_32 = 53.12;
let g as Float_64 = 1_200.500_1;  // underscores in floats
```

### Void and Any

```citrus
// used as return types only — not valid for variables

calculate() -> Void { }     // returns nothing
box_it<T>() -> Any { }      // returns any type — use sparingly
```

---

## References and Borrowing

```citrus
// immutable reference
let r as &Int_32 = &x;

// mutable reference — original must be mutable
let mutable x as Int_32 = 5;
let r as &mutable Int_32 = &mutable x;

// passing to functions
call_fn(&x);
call_fn(&mutable x);
```

### Rules

- You can have **many immutable references** OR **one mutable reference** — never both at the same time
- References cannot outlive the value they point to
- Follows Rust borrow checker semantics exactly

---

## Operators

### Math
```
+    -    *    /    %
+=   -=   *=   /=   %=
```

### Comparison
```
==   !=   <   >   <=   >=
```

### Logical
```
&&   ||   !
```

### Bitwise
```
&    |    ^    ~    <<    >>
```

### Other
```
=       assignment
->      return type arrow
=>      fat arrow (shorthand lambda body)
?       error propagation
..      exclusive range     0..5  means 0,1,2,3,4
..=     inclusive range     0..=5 means 0,1,2,3,4,5
&       reference / bitwise AND
.       member access
::      path separator
@       attribute prefix
```

---

## Functions

### Syntax
```
name(param as Type, ...) -> ReturnType {
    return value;
}
```

### Examples

```citrus
// entry point
main() -> UInt_32 {
    return 0;
}

// typed params and return
add(x as Int_32, y as Int_32) -> Int_32 {
    return x + y;
}

// no return value
log(msg as Text) -> Void {
    println!("{}", msg);
}

// returning Result
read_file(path as Text) -> Result<Text, Text> {
    let content = load(path)?;
    return Ok(content);
}
```

### Calling

```citrus
add(10, 20);
add(x=10, y=20);         // named arguments
process(&value);         // pass by reference
process(&mutable value); // pass mutable reference
```

### Rules
- Return type is always required — use `Void` if nothing returned
- Parameter types are always required — no inference in function signatures
- `return` is explicit — no implicit last-expression return

---

## Anonymous Functions

### Syntax
```
[capture](params) -> RetType { body }
[capture](params) => expression     // shorthand
```

### Capture Clause

The `[...]` before params defines how outer variables are captured.

| Syntax         | Meaning                                             |
|----------------|-----------------------------------------------------|
| `[]`           | Copy all — copiable types copied, others compile error |
| `[&]`          | Borrow all by immutable reference — **recommended default** |
| `[&mutable]`   | Borrow all by mutable reference                     |
| `[=]`          | Move all — ownership transferred in                 |
| `[&a, =b, c]`  | Per-variable: borrow a, move b, copy c              |

> **Convention:** Use `[&]` as your default capture. Only use `[]` when you genuinely need independent copies. Non-copyable types inside `[]` are a compile error unless the type implements the `Clone` trait.

### Examples

```citrus
// recommended default capture
let double = [&](x as Int_32) => x * 2;

// full form
let add = [&](x as Int_32, y as Int_32) -> Int_32 {
    return x + y;
};

// copy capture
let multiplier = 3;
let scale = [](x as Int_32) => x * multiplier;

// move capture
let name = "Citrus";
let greet = [=]() -> Text {
    return name;   // name is moved in — original no longer valid
};

// per-variable capture
let threshold = 10;
let label     = "high";
let check = [&threshold, =label](x as Int_32) => x > threshold;

// passing as argument
items.map([&](x) => x * 2);
items.filter([&](x) => x > 0);
```

---

## Structs

```citrus
// definition — always PascalCase
struct Animal {
    name   as Text,
    height as Int_32
}

// generic struct
struct Box<T> {
    value as T
}

// instantiation — custom types must always be annotated
let a as Animal = Animal {
    name: "Lion",
    height: 120
};

let b as Box<Int_32> = Box { value: 42 };

// member access
a.name;
a.height;
```

### Rules
- Structs are **data layouts only** — no methods in the struct body
- Methods go in `implement` blocks
- No inheritance between structs

---

## Enums

### Simple enum
```citrus
enum Direction {
    North,
    South,
    East,
    West
}

let dir as Direction = Direction::North;
```

### With explicit integer values
```citrus
enum Status {
    Active   = 1,
    Inactive = 2,
    Pending  = 3
}

let s as Status = Status::Active;
```

### With data — tagged union
```citrus
enum Shape {
    Circle(Float_32),
    Rectangle(Float_32, Float_32),
    Point
}

let s as Shape = Shape::Circle(5.0);
let r as Shape = Shape::Rectangle(10.0, 4.5);
let p as Shape = Shape::Point;
```

### With named fields
```citrus
enum Message {
    Quit,
    Move  { x as Int_32, y as Int_32 },
    Write(Text),
    Color(UInt_8, UInt_8, UInt_8)
}

let m as Message = Message::Move { x: 10, y: 20 };
```

### Generic enum
```citrus
// how Option and Result are built internally
enum Option<T> {
    Some(T),
    None
}

enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

### Methods on enums
```citrus
implement Direction {
    is_vertical(self) -> Bool {
        match self {
            Direction::North | Direction::South => { return true; }
            _ => { return false; }
        }
    }

    opposite(self) -> Direction {
        match self {
            Direction::North => { return Direction::South; }
            Direction::South => { return Direction::North; }
            Direction::East  => { return Direction::West; }
            Direction::West  => { return Direction::East; }
        }
    }
}
```

### Implement traits on enums
```citrus
implement Describe for Direction {
    describe(self) -> Text {
        match self {
            Direction::North => { return "Heading north"; }
            Direction::South => { return "Heading south"; }
            Direction::East  => { return "Heading east"; }
            Direction::West  => { return "Heading west"; }
        }
    }
}
```

### Match on enums with data
```citrus
match shape {
    Shape::Circle(r)       => { println!("Circle r={}", r); }
    Shape::Rectangle(w, h) => { println!("{}x{}", w, h); }
    Shape::Point           => { println!("A point"); }
}

// named fields
match msg {
    Message::Move { x, y } => { println!("Move to {},{}", x, y); }
    Message::Write(text)   => { println!("{}", text); }
    Message::Quit          => { println!("Quit"); }
    _                      => { }
}
```

### Visibility
```citrus
public enum Direction {
    North,
    South,
    East,
    West
}
```

### Rules
- Enum variants are accessed via `::` — `Direction::North`
- Variants can hold no data, tuple data, or named fields
- Methods and trait implementations go in `implement` blocks
- Generic enums follow the same rules as generic structs — always annotate explicitly

---

## Traits

```citrus
// define a trait
trait Speak {
    speak(self) -> Void;
}

trait Walk {
    walk(self, steps as UInt_32) -> Void;
}

// trait with default implementation
trait Describe {
    describe(self) -> Text;

    print_description(self) -> Void {
        println!("{}", self.describe());
    }
}
```

---

## Implement

```citrus
// implement methods on a struct
implement Animal {
    new(name as Text, height as Int_32) -> Animal {
        return Animal { name: name, height: height };
    }

    get_name(self) -> &Text {
        return &self.name;
    }

    set_height(mutable self, h as Int_32) -> Void {
        self.height = h;
    }
}

// implement a trait for a struct
implement Speak for Animal {
    speak(self) -> Void {
        println!("{} speaks", self.name);
    }
}

// implement multiple traits
implement Walk for Animal {
    walk(self, steps as UInt_32) -> Void {
        println!("Walking {} steps", steps);
    }
}
```

---

## Generics

```citrus
// generic function
identity<T>(value as T) -> T {
    return value;
}

// with single trait bound
transform<T>(item as T) -> T where T implements Speak {
    item.speak();
    return item;
}

// with multiple trait bounds
process<T>(item as T) -> T where T implements Speak + Walk {
    item.speak();
    item.walk(10);
    return item;
}

// multiple generic params
zip<A, B>(a as A, b as B) -> Box<A> where A implements Clone {
    return Box { value: a };
}

// calling — must always type generics explicitly
let result = identity<Int_32>(42);
transform<Animal>(my_animal);
```

---

## Option Type

Represents a value that may or may not exist. No null in Citrus — use `Option` instead.

```citrus
// wrapping a value
let score  as Option<Int_32> = Some(42);
let absent as Option<Int_32> = None;

// unwrapping with match — recommended
match score {
    Some(v) => { println!("Score: {}", v); }
    None    => { println!("No score"); }
}

// unwrapping with if let
if let Some(v) = score {
    println!("Got {}", v);
}

// chaining with ?  inside a function returning Option
find_player(id as UInt_32) -> Option<Text> {
    let player = players.get(id)?;    // returns None early if not found
    return Some(player.name);
}
```

---

## Error Handling

Uses `Result<T, E>` — either a success value `Ok(T)` or an error `Err(E)`.

```citrus
// constructing
let ok  as Result<Int_32, Text> = Ok(42);
let err as Result<Int_32, Text> = Err("something went wrong");

// handling with match
match result {
    Ok(value) => { println!("Got {}", value); }
    Err(e)    => { println!("Error: {}", e); }
}

// ? operator — propagate error to caller
// function must return Result to use ?
parse(input as Text) -> Result<Int_32, Text> {
    let cleaned = sanitize(input)?;    // returns Err early if sanitize fails
    let parsed  = to_int(cleaned)?;    // same
    return Ok(parsed);
}

// combining Option and Result
load_score(path as Text) -> Result<Option<Int_32>, Text> {
    let content = read_file(path)?;
    let score   = parse_score(content);   // returns Option<Int_32>
    return Ok(score);
}
```

---

## Control Flow

### If / Else

```citrus
if x > 100 {
    // ...
} else if x > 50 {
    // ...
} else {
    // ...
}
```

### While

```citrus
while x < 100 {
    x += 1;
}
```

### Loop

```citrus
// infinite loop — exit with break
loop {
    if done { break; }
    if skip { continue; }
}
```

### For-In

```citrus
// range — exclusive
for i in 0..5 { }         // 0 1 2 3 4

// range — inclusive
for i in 0..=5 { }        // 0 1 2 3 4 5

// over a vector
for item in items { }

// over an array
for score in scores { }

// over chars of text
for ch in message { }

// with index via enumerate
for i, item in items.enumerate() { }

// with variable range
for i in start..end { }
```

### Match

```citrus
match value {
    0      => { println!("zero"); }
    1 | 2  => { println!("one or two"); }
    3..=9  => { println!("three to nine"); }
    _      => { println!("other"); }
}

// match on Option
match maybe_value {
    Some(v) => { }
    None    => { }
}

// match on Result
match result {
    Ok(v)  => { }
    Err(e) => { }
}
```

---

## Iterators and Ranges

### Iterator Trait

Any type that implements `Iterator` can be used in `for-in` and iterator adapters.

```citrus
trait Iterator<T> {
    next(mutable self) -> Option<T>;
}

trait Iterable<T> {
    iter(self) -> Iterator<T>;
}
```

### Types that are Iterable

- `Vector<T>`
- `[T:N]` (arrays)
- `Range`
- `RangeInclusive`
- `Text` (iterates over `Char`)

### Ranges

```citrus
0..5             // Range — exclusive
0..=5            // RangeInclusive — inclusive

// stored as values
let r as Range = 0..10;
let r as RangeInclusive = 1..=10;

// used in match
match score {
    0..=59  => { println!("fail"); }
    60..=79 => { println!("pass"); }
    80..=100 => { println!("merit"); }
    _        => { }
}
```

### Iterator Adapters

```citrus
items.map([&](x) => x * 2)
items.filter([&](x) => x > 0)
items.reduce([&](acc, x) => acc + x)
items.enumerate()                      // yields (index, value)
items.zip(other)                       // pairs two iterables
items.take(5)                          // first 5 items
items.skip(2)                          // skip first 2
items.collect<Vector<Int_32>>()        // gather into collection
items.any([&](x) => x > 10)           // true if any match
items.all([&](x) => x > 0)            // true if all match
items.find([&](x) => x == 5)          // first matching — Option<T>
items.count()                          // number of items
```

---

## Arrays and Vectors

### Arrays — fixed size

Size is part of the type. Stack or heap depending on context.

```citrus
let scores as [UInt_8:5] = [1, 2, 3, 4, 5];
let zeros  as [Int_32:3] = [0, 0, 0];

// access
let first = scores[0];
scores[1] = 99;       // only if scores is mutable

// in a for loop
for score in scores { }
```

### Vectors — dynamic size

Heap allocated. Growable. Built on `macro` + `struct` + `Iterator`.

```citrus
let items as Vector<UInt_8>  = Vector![1, 2, 3];
let names as Vector<Text>    = Vector!["Alice", "Bob"];
let empty as Vector<Int_32>  = Vector![];

// access
let first = items[0];

// methods
items.push(4);
items.pop();                    // Option<T>
items.len();                    // UInt_64
items.is_empty();               // Bool
items.get(2);                   // Option<T> — safe access
items.contains(&value);         // Bool
items.remove(1);                // removes at index
items.clear();
```

---

## Macros

### Defining a Macro

```citrus
macro log!(message) {
    // expands at compile time
}

macro assert!(condition, message) {
    // ...
}

macro make_struct!(name, field, field_type) {
    // generates a struct definition
}
```

### Calling a Macro

Any delimiter works — `()`, `[]`, `{}`:

```citrus
log!("hello");
log!["hello"];
log!{"hello"};
```

Convention:
- `!()` — for expression-like calls
- `![]` — for collection-like calls `Vector![1, 2, 3]`
- `!{}` — for block-like or multi-statement expansions

### Built-in Macros

```citrus
println!("Hello {}", name)      // print with newline
print!("Hello {}", name)        // print without newline
format!("value: {}", x)         // build a Text value
panic!("unrecoverable error")   // halt immediately
assert!(x > 0, "must be positive")
Vector![1, 2, 3]
```

---

## Attributes

Applied with `@` prefix before any item.

```citrus
// derive common trait implementations
@derive(Debug, Clone, Eq)
struct Point {
    x as Float_32,
    y as Float_32
}

// mark as deprecated
@deprecated("use new_calculate instead")
old_calculate(x as Int_32) -> Int_32 {
    return x;
}

// inline hint to compiler
@inline
fast_add(x as Int_32, y as Int_32) -> Int_32 {
    return x + y;
}

// custom attribute macros
@route("GET", "/users")
get_users() -> Result<Vector<Text>, Text> { }

@test
test_addition() -> Void {
    assert!(add(1, 2) == 3, "addition failed");
}
```

---

## Modules and Imports

### File-Based Modules

Every `.cit` file is automatically a module named after the file.

```
src/
  main.cit        → module main
  math.cit        → module math
  animals.cit     → module animals
```

### Declaring a Module Inline

```citrus
module geometry {
    public struct Point {
        x as Float_32,
        y as Float_32
    }

    public distance(a as &Point, b as &Point) -> Float_32 {
        // ...
    }
}
```

### Importing

`::` is the path separator.

```citrus
import math::calculate;              // snake_case — function or variable
import animals::Animal;              // PascalCase — struct or trait
import animals::Speak;               // PascalCase — trait
import geometry::Point;

// multiple from same module
import animals::{Animal, Speak, Walk};

// everything public from a module
import animals::*;

// nested modules
import engine::http::Request;
```

### Convention
- `PascalCase` after `::` → struct or trait
- `snake_case` after `::` → function or variable
- `UPPER_SNAKE` after `::` → constant

---

## Visibility

Everything is **module-private by default**.

```citrus
// public struct
public struct Animal {
    name   as Text,
    height as Int_32
}

// public trait
public trait Speak {
    speak(self) -> Void;
}

// public function
public calculate(x as Int_32) -> Int_32 {
    return x * 2;
}

// public constant
public static MAX_PLAYERS as UInt_32 = 64;

// public implement — the methods you mark public are accessible
implement Animal {
    public get_name(self) -> &Text {
        return &self.name;
    }

    // private — only accessible within the module
    internal_reset(mutable self) -> Void {
        self.height = 0;
    }
}
```

---

## Ownership Rules Summary

Citrus ownership follows Rust semantics exactly.

### Ownership
- Every value has exactly **one owner**
- When the owner goes out of scope, the value is dropped (memory freed)
- Assignment **moves** ownership for non-Copy types

```citrus
let a as Text = "hello";
let b = a;             // a is MOVED into b — a is no longer valid
println!("{}", a);     // COMPILE ERROR — a was moved
```

### Copy Types
These types are cheap enough to copy automatically on assignment:

```
Bool   Char   
Int_8  Int_32  Int_64  Int_128
UInt_8 UInt_32 UInt_64 UInt_128
Float_32  Float_64
```

```citrus
let a as Int_32 = 5;
let b = a;    // a is COPIED — both a and b are valid
```

### Clone
Non-copy types must explicitly clone:

```citrus
let a as Text = "hello";
let b = a.clone();    // explicit clone — both valid
```

### Borrowing
```citrus
// immutable borrow — many allowed at once
let r1 = &x;
let r2 = &x;     // fine

// mutable borrow — only one at a time
let r = &mutable x;
// no other reference to x allowed while r exists

// cannot mix — if mutable borrow exists, no immutable borrows allowed
```

### Lifetimes
References cannot outlive the value they point to. The compiler enforces this. Explicit lifetime annotations are not part of Citrus — the compiler infers them.

---

## Quick Reference Card

```
// variable
let name as Type = value;
let mutable name as Type = value;
static NAME as Type = value;

// function
fn_name(param as Type) -> RetType { return value; }

// struct
struct Name { field as Type }

// enum
enum Name { VariantA, VariantB(Type), VariantC { field as Type } }
let v as Name = Name::VariantA;

// implement
implement Name { method(self) -> Type { } }

// trait
trait Name { method(self) -> Type; }

// implement trait
implement Trait for Struct { }

// generics
fn<T>(x as T) -> T where T implements Trait { }

// lambda
[&](x as Type) => expression
[&](x as Type) -> RetType { return value; }

// option
Some(value)   None
// result
Ok(value)     Err(error)

// ranges
0..5    0..=5

// import
import module::Name;

// macro call
name!(args)   name![args]   name!{args}

// attribute
@derive(Trait)
@deprecated("message")
```

---

*Citrus — simple, safe, expressive.*
