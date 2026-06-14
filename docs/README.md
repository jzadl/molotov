# Molotov

Molotov is a programming language with Python-like syntax that compiles to native binaries via Rust. Write in Python-style syntax, get Rust performance.


```python
def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

for i in range(10):
    print(fib(i))
```

## Quick start

### Install

For Linux/macOS:
```bash
chmod +x install.sh && ./install.sh
```

For Windows (PowerShell):
```powershell
.\install.ps1
```

Or using Cargo (mltv only):
```bash
cargo install --path .
```

### Compile and run

```bash
mltv deploy hello.mltv -o hello
./hello
```

### Or run in one step

```bash
mltv run hello.mltv
```

### Shorthand for run

```bash
mltv hello.mltv
```

### CLI

```
mltv deploy <file>          Transpile .mltv and compile to binary
  -o, --output <name>       Output binary name (default: a.out)
  --rust-only               Only generate .rs file, skip compilation
  --keep                    Keep the generated .rs file

mltv run <file>             Transpile, compile, and run in one step

### Package Manager (mpm)

```
mpm install <user/repo>     Install a library from GitHub
```

Check the [MPM Guide](mpm-guide.md) for more details.

## Language guide

### Basics

```
# Comments use #
print("Hello, Molotov!")
```

### Variables

Variables are dynamically typed (inference happens at compile time):

```
name = "Alice"
age = 30
pi = 3.14
```

### Functions

```
def greet(name):
    return "Hello, " + name
```

Functions that use `raise` automatically get a `Result<T, String>` return type.

### Control flow

```
if x > 0:
    print("positive")
elif x == 0:
    print("zero")
else:
    print("negative")
```

### Loops

```
for i in range(10):
    print(i)

while x > 0:
    x = x - 1
```

For-loop destructuring:

```
for k, v in pairs:
    print(k)
    print(v)
```

### Lists

```
numbers = [1, 2, 3, 4, 5]
numbers.append(6)
print(len(numbers))
for n in numbers:
    print(n)
```

### List comprehensions

```
squares = [x * x for x in range(10)]
evens = [x for x in range(20) if x % 2 == 0]
```

### Dictionaries

```
person = {"name": "Alice", "age": "30"}
print(person["name"])
person["age"] = "31"
```

### Dict comprehensions

```
squares = {x: x * x for x in range(5)}
```

### Slicing

```
text = "hello world"
print(text[0:5])
print(text[6:])
print(text[:5])
```

### Classes

```
class Dog:
    def __init__(self, name, age):
        self.name = name
        self.age = age

    def bark(self):
        print("Woof! I am " + self.name)

d = Dog("Buddy", 3)
d.bark()
```

### try / except / raise

```
def divide(a, b):
    if b == 0:
        raise "division by zero"
    return a / b

try:
    print(divide(10, 0))
except:
    print("caught an error")
```

### with

```
with open("file.txt", "r"):
    print("ok")
```

### Decorators

```
def log(fn):
    def wrapped(n):
        print("called with " + str(n))
        return fn(n)
    return wrapped

@log
def double(x):
    return x * 2
```

### Lambdas

```
f = lambda x: x + 1
print(f(5))
```

### f-strings

```
name = "world"
print(f"hello {name}")
```

### Import

```
import math
from os import path
import math as m
from os import path as p
```

### Augmented assignment

```
x += 1
y -= 2
z *= 3
w /= 4
```

### Embed raw Rust

```
embed_rust('println!("from rust!");')
```

Or from a file:

```
cinclude("helper.rs")
```

## Built-in functions

| Function | Description |
|----------|-------------|
| `print(x)` | Print value |
| `len(x)` | Length of list/string |
| `range(n)` | Range 0..n |
| `range(s, e)` | Range s..e |
| `range(s, e, step)` | Range with step |
| `int(x)` | Parse to integer |
| `float(x)` | Parse to float |
| `str(x)` | Convert to string |
| `input(prompt)` | Read line from stdin |
| `sleep(ms)` | Sleep for milliseconds |
| `randint(lo, hi)` | Random integer in range |
| `randch(list)` | Random element from list |
| `shuffle(list)` | Shuffle list in place |
| `sum(list)` | Sum of numbers |
| `avg(list)` | Average of numbers |
| `min_val(a, b)` / `min_val(list)` | Minimum |
| `max_val(a, b)` / `max_val(list)` | Maximum |
| `clamp(x, lo, hi)` | Clamp value |
| `abs(x)` | Absolute value |
| `round(x)` | Round float |
| `type_of(x)` | Type as string |
| `read_file(path)` | Read file to string |
| `write_file(path, data)` | Write string to file |
| `exists(path)` | Check if file exists |
| `today()` | Today's date string |
| `now()` | Current time string |
| `clear()` | Clear console |
| `map(fn, list)` | Map over list |
| `filter(fn, list)` | Filter list |
| `enumerate(list)` | Enumerate (returns pairs) |
| `zip(a, b)` | Zip two lists |
| `args()` | Get command-line arguments |
| `embed_rust(code)` | Inline Rust code |
| `cinclude(path)` | Include Rust file |

## VSCode extension

The `vscode-molotov/` directory contains a VSCode extension providing syntax highlighting and file icons for `.mltv` files.

### Install locally

```bash
code --install-extension vscode-molotov/
```

## How it works

1. **Tokenizer** breaks source into tokens with indentation tracking
2. **Parser** builds an AST using recursive descent
3. **Transpiler** generates Rust code with type inference
4. **rustc** compiles the Rust code to a native binary

   my first rust program, learned after like 1 year or something like that
