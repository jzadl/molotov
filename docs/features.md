# Molotov Language Features

## Syntax
Molotov uses Python-style indentation-based syntax. If you know Python, you know Molotov.

## Available Features

### Variables & Types
```python
x = 5              # i64
y = 3.14           # f64
name = "Alice"     # String
flag = True        # bool
nothing = None     # unit type ()
```

### Tuples & Unpacking
```python
a, b = 1, 2
a, b = b, a       # swap!
t = (1, 2, 3)
print(t[0])        # tuple indexing → 1
```

### Math
```python
+  -  *  /  //  %  **
==  !=  <  >  <=  >=
and  or  not  in  not in  is  is not
```

### Chained Comparisons
```python
if 0 < x < 10:       # equivalent to: 0 < x and x < 10
    print("in range")
```

### If / Elif / Else
```python
if x > 0:
    print("positive")
elif x < 0:
    print("negative")
else:
    print("zero")
```

### Loops
```python
for i in range(10):
    print(i)

while x > 0:
    x = x - 1
```

### Functions
```python
def add(a, b):
    return a + b

# Lambda
double = lambda x: x * 2
```

### Classes
```python
class Dog:
    def __init__(self, name, age):
        self.name = name
        self.age = age

    def bark(self):
        print("Woof!")

buddy = Dog("Buddy", 3)
```

### Imports
```python
import math
import firebase as fb
from json import loads, dumps
```

### Lists & Dicts
```python
nums = [1, 2, 3]
nums.append(4)
print(nums[0])

scores = {"alice": 95, "bob": 87}
print(scores["alice"])
```

### Comprehensions
```python
squares = [n * n for n in range(10)]
evens = [n for n in range(20) if n % 2 == 0]
squares_dict = {n: n * n for n in range(5)}
```

### Try / Except / Raise
```python
try:
    risky_stuff()
except:
    print("caught it")
raise "something went wrong"
```

### With Statement
```python
with open("file.txt"):
    print("reading")
```

### Decorators
```python
@log
def compute(x):
    return x * 2
```

### Enumerate & Zip
```python
for item in enumerate(range(5)):
    print(item.0, item.1)

for pair in zip(list_a, list_b):
    print(pair.0, pair.1)
```

### Lambdas in Map / Filter
```python
doubled = map(lambda x: x * 2, nums)
evens = filter(lambda x: x % 2 == 0, nums)
```

---

## Built-in Functions

| Function | What it does | Example |
|---|---|---|
| `print(x)` | Print a value | `print("hello")` |
| `len(x)` | Length of list/string/dict | `len(items)` |
| `range(n)` / `range(a, b)` / `range(a, b, step)` | Number range | `range(10)` |
| `int(x)` | Convert to integer | `int("42")` |
| `float(x)` | Convert to float | `float("3.14")` |
| `str(x)` | Convert to string | `str(42)` |
| `input(msg)` | Read user input | `name = input("Name: ")` |
| `args()` | Get CLI arguments | `for a in args(): print(a)` |

### 🧰 Utility Functions

| Function | What it does | Example |
|---|---|---|
| `abs(n)` | Absolute value | `abs(-5)` → 5 |
| `round(n)` | Round float to nearest int | `round(3.7)` → 4 |
| `min_val(a, b)` | Smaller of two | `min_val(3, 7)` → 3 |
| `max_val(a, b)` | Larger of two | `max_val(3, 7)` → 7 |
| `clamp(val, lo, hi)` | Keep value between bounds | `clamp(15, 0, 10)` → 10 |
| `sum(list)` | Sum of numbers | `sum([1, 2, 3])` → 6 |
| `avg(list)` | Average of numbers | `avg([1, 2, 3])` → 2.0 |
| `sleep(secs)` | Pause execution | `sleep(1.5)` |
| `clear()` | Clear terminal screen | `clear()` |
| `type_of(x)` | Get Rust type name of value | `type_of(42)` → `"i64"` |
| `all(list)` | True if all elements are truthy | `all([True, False])` → False |
| `any(list)` | True if any element is truthy | `any([False, True])` → True |
| `sorted(list)` | Return sorted copy | `sorted([3, 1, 2])` → [1, 2, 3] |
| `reversed(list)` | Return reversed copy | `reversed([1, 2, 3])` → [3, 2, 1] |

### 🎲 Random Functions

| Function | What it does | Example |
|---|---|---|
| `randint(lo, hi)` | Random integer in range | `randint(1, 100)` |
| `randch(list)` | Pick random element | `randch(items)` |
| `shuffle(list)` | Shuffle list in place | `shuffle(items)` |

### 📁 File Functions

| Function | What it does | Example |
|---|---|---|
| `read_file(path)` | Read file as string | `text = read_file("data.txt")` |
| `write_file(path, content)` | Write string to file | `write_file("out.txt", "hello")` |
| `exists(path)` | Check if file exists | `exists("data.txt")` → True/False |

### 📅 Date / Time

| Function | What it does | Example |
|---|---|---|
| `today()` | Current date as YYYY-MM-DD | `today()` → `"2026-06-13"` |
| `now()` | Current time as HH:MM:SS | `now()` → `"14:30:00"` |

### ⚡ Compile-Time Features

| Function | What it does | Example |
|---|---|---|
| `embed_rust("code")` | Inline raw Rust code | `embed_rust("println!(\"hi\");")` |
| `cinclude("path")` | Embed file at compile time | `text = cinclude("data.txt")` |

String methods: `upper`, `lower`, `strip`, `split`, `join`, `replace`, `startswith`, `endswith`, `find`, `capitalize`, `title`, `swapcase`, `splitlines`, `rsplit`, `lstrip`, `rstrip`, `isalpha`, `isdigit`, `isalnum`, `isspace`, `islower`, `isupper`, `count`, `index`, `rfind`, `rindex`

List methods: `append`, `pop`, `insert`, `remove`, `sort`, `reverse`, `clear`, `copy`

Dict methods: `keys`, `values`, `items`, `get`, `pop(key, default?)`, `popitem`, `clear`

### 🔮 Internal (you can call these but they have magic behavior)
- `enumerate(iter)` → `.enumerate()` call
- `zip(a, b, ...)` → `.zip()` chaining
- `map(f, iter)` → `.iter().map()`
- `filter(f, iter)` → `.into_iter().filter()`

---

## Running Scripts and CLI Arguments

### Direct Execution
You can run any `.mltv` file directly with the `mltv` command:
```bash
mltv script.mltv
```

### Shebang Support
You can add a shebang to the top of your script to make it executable:
```python
#!/usr/bin/env mltv
print("Hello from a shebang script!")
```
Then, after giving it execution permissions (`chmod +x script.mltv`), you can run it as:
```bash
./script.mltv
```

### Passing Arguments
To pass arguments to your script, use the `--` separator:
```bash
mltv script.mltv -- hello world 123
```
Inside your script, access these arguments using `args()`:
```python
all_args = args()
# all_args[0] is the binary path
# all_args[1] is "hello"
# all_args[2] is "world"
# all_args[3] is "123"
```
