# hson
JSON like format for HTML.  
The parser can be used for other purposes, but its main goal is to represent a HTML structure in a JSON style.

## Main differences with standard json
* Allow same key multiple times in same object
* Does not allow array of objects

## Usage
   [Parsing](#Parsing)  
   [Stringifying](#Stringifying)  
   [Querying](#Querying)  
   [Inserting](#Inserting)  
   [Removing](#Removing)  
   [Iterating](#Iterating)  
   [Debugging](#Debugging)  
   [Events listening](#Events)
   
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
hson.print_nodes(true);
```

### Stringifying
```rust
...
  
let s = hson.stringify();
println!("{}", &s);
```

### Querying
#### Find matching nodes
Querying works as the javascript querySelectorAll method but without the 'CSS' features.  
(Queries improvement is planned).
```rust
use hson::{ Hson, Query };
  
...
  
// Get node's identifiers for easier processing
let results = hson.query("div p").unwrap();
println!("\n{:?}\n", results);
  
// Get node's reference
let results = hson.query_nodes("div p").unwrap();
println!("\n{:?}\n", results);
  
// Recursive search in a node
let results = hson.query_on(&uid, "div p", true).unwrap();
println!("\n{:?}\n", results);
  
// Non recursive search in a node
let results = hson.query_on(&uid, "attrs", false).unwrap();
println!("\n{:?}\n", results);
  
// Recursive search in a node and get node's reference
let results = hson.query_on_nodes(&node, "div p", true).unwrap();
println!("\n{:?}\n", results);
  
// Non recursive search in a node and get node's reference
let results = hson.query_on_nodes(&node, "attrs", false).unwrap();
println!("\n{:?}\n", results);
  
// Get node key
let key = hson.get_node_key(&node);
  
// Get node value
let value = hson.get_node_value(&node);
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
  
hson.insert(&results[0], 1, child).unwrap();
hson.print_data(true);
```

### Removing
```rust
use hson::{ Hson, Query, Ops, Debug };
  
...
  
let results = hson.query("p").unwrap();

hson.remove(&results[0]).unwrap();
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
  
fn on_event (evt: Event, uid: String) {
    println!("\nEVENT : {:?} on {}\n", evt, uid);
}
  
let mut hson = Hson::new();
hson.subscribe(on_event);
  
...
```

