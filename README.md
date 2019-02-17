# hson
JSON like format for HTML.  
The parser can be used for other purposes, but its main goal is to represent a HTML structure in a JSON style.  
Allow to query the data the same way the DOM is queried client-side through `QuerySelectorAll` method. 

## Main differences with standard json
* Allow same key multiple times in same object
* Does not allow `null` and `undefined`

## Usage
   [Parsing](#Parsing)  
   [Stringifying](#Stringifying)   
   [Searching](#Searching)  
   [Inserting](#Inserting)  
   [Removing](#Removing)  
   [Iterating](#Iterating)  
   [Debugging](#Debugging)  
   [Events listening](#Events)  
   [Nodes manipulation](#Manipulation)
   
### Parsing
```rust
use hson::{ Hson, Debug };
  
...
  
let data = r#"{
                "div": {
                  "attrs": {
                    "class": [""],
                    "onClick": "doSomething"
                  },
                  "div": {
                    "p": {
                      "attrs": {},
                      "span": {
                        "text": "Hello"
                      }
                    },
                    "p": {}
                  },
                  "div": {
                    "component": "test",
                    "attrs": {},
                    "onClick": "componentDoSomething"
                  }
                }
              }"#;
              
let mut hson = Hson::new();
hson.parse(&data).unwrap();
hson.print_data(true);
```

### Stringifying
```rust
...
  
let s = hson.stringify();
println!("{}", &s);
```

### Searching
Search is similar to the javascript `querySelectorAll` method.  
```rust
use hson::{ Hson, Query, Search };
  
...
  
// Standard recursive search
let results = hson.search("div p").unwrap();
println!("\n{:?}\n", results);
  
// Look for immediate childs, no recursive search
let results = hson.search("div>p").unwrap();
println!("\n{:?}\n", results);
  
// Look for multiple childs (OR search)
let results = hson.search("attrs id|rate|trusted").unwrap();
println!("\n{:?}\n", results);
  
// Look for a node with a specific value
let results = hson.search("attrs id='12'").unwrap();
println!("\n{:?}\n", results);
  
// All those features can be combined
let results = hson.search("div>p attrs id='12'|rate='3'|trusted").unwrap();
println!("\n{:?}\n", results);
  
// Look for childs in a specific node, no recursion
let results = hson.search_in(node_id, "div>p").unwrap();
println!("\n{:?}\n", results);
  
let results = hson.search_in(node_id, ">p").unwrap();
println!("\n{:?}\n", results);
```

### Inserting
```rust
use hson::{ Hson, Query, Ops, Debug };
  
...
  
let results = hson.query("div p").unwrap();
let child = r#"{
                    "i": {
                        "class": [],
                        "text": "World"
                    },
                    "ul": {
                        "class": ["active","test"]
                    }
                }"#;
  
hson.insert(results[0], 1, child).unwrap();
hson.print_data(true);
```

### Removing
```rust
use hson::{ Hson, Query, Ops, Debug };
  
...
  
let results = hson.query("p").unwrap();

hson.remove(results[0]).unwrap();
hson.print_data(true);
```

### Iterating
Iterate over the nodes identifiers
```rust
...
  
for id in hson {
    println!("{}", id);
}
  
// OR
loop {
    let id = match hson.next() {
        Some(s) => s,
        None => break
    };
  
    match &hson.nodes.get(&id) {
        Some(node) => {
            println ! ("{} : {}", node.instance, node.id);
        }
        None => {
            break
        }
    }
}
```

### Debugging
```rust
use hson::{ Hson, Debug };
  
...
  
hson.print_process_time();
hson.print_nodes(true); // true for sorted printing
hson.print_data(true); // true for pretty printing
```

### Events
Current supported events are _Parse_, _Insert_, _Remove_.
```rust
use hson::{ Hson, Ops, Event };
  
...
  
fn on_event (evt: Event, uid: u64) {
    println!("\nEVENT : {:?} on {}\n", evt, uid);
}
  
let mut hson = Hson::new();
hson.subscribe(on_event);
  
...
```

### Manipulation
Nodes values can be casted to primitive types using `Vertex`, a `Node` clone with more attributes.  
_**Note : Vertex are Nodes clones and not references to the underlying Nodes. Manipulating Vertex's values will not be reflected on their matching Nodes.**_
```rust
use hson::{ Hson, Query, Search, Cast };
  
...
  
let results = hson.search("div attrs class").unwrap();
let vertex = hson.get_vertex(results[0]).unwrap();
  
// Get vertex value as u64
println!("{}", vertex.value_as_f64());
  
// Get vertex value as a vector of String
println!("{:?}", vertex.value_as_array());
  
// Cast a string value to a different type
let s = "0.456";
println!("{}", vertex.as_f64(s));
```

##### Vertex methods
* `fn key_as_string (&self) -> Option<String>`
* `fn key_as_f64 (&self) -> Option<f64>`
* `fn key_as_i64 (&self) -> Option<i64>`
* `fn key_as_u64 (&self) -> Option<u64>`
* `fn key_as_bool (&self) -> Option<bool>`
* `fn value_as_string (&self) -> Option<String>`
* `fn value_as_f64 (&self) -> Option<f64>`
* `fn value_as_i64 (&self) -> Option<i64>`
* `fn value_as_u64 (&self) -> Option<u64>`
* `fn value_as_bool (&self) -> Option<bool>`
* `fn value_as_array (&self) -> Option<Vec<String>>`
* `fn as_f64 (&self, value: &str) -> Option<f64>`
* `fn as_i64 (&self, value: &str) -> Option<i64>`
* `fn as_u64 (&self, value: &str) -> Option<u64>`
* `fn as_bool (&self, value: &str) -> Option<bool>`
  
