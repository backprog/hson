#![allow(unused_assignments)]

extern crate uuid;

use std::collections::HashMap;
use std::vec::Vec;
use std::iter::FromIterator;
use std::io::{ ErrorKind, Error };
use std::time::{ Instant };

use uuid::Uuid;


type Callback = fn(Event, String);

const OPEN_CURLY: char = '{';
const CLOSE_CURLY: char = '}';
const OPEN_ARR: char = '[';
const CLOSE_ARR: char = ']';
const DOUBLE_QUOTES: char = '"';
const COLONS: char = ':';
const COMMA: char = ',';

/// Events types
#[derive(PartialEq, Clone, Debug)]
pub enum Event {
    Parse,
    Insert,
    Remove
}

/// Node types
#[derive(PartialEq, Clone, Debug)]
pub enum Kind {
    Node,
    Array,
    Integer,
    Float,
    String,
    Bool,
    Null
}

/// Hson node
#[derive(Clone, Debug)]
pub struct Node {
    pub root: bool,
    pub kind: Kind,
    pub parent: String,
    pub childs: Vec<String>,
    pub key: [usize; 2],
    pub value: [usize; 2],
    pub id: String,
    pub opened: bool,
    pub instance: u32
}

/// Hson cloned node
#[derive(Clone, Debug)]
pub struct Vertex {
    pub root: bool,
    pub kind: Kind,
    pub parent: String,
    pub childs: Vec<String>,
    pub id: String,
    pub instance: u32,
    pub key: String,
    pub value: String
}

/// Controls chars
struct Controls {
    chars: [char; 7],
    curly_brackets: u16,
    square_brackets: u16,
    double_quotes: u16
}

/// Hson format
pub struct Hson {
    data: Vec<char>,
    pub nodes: HashMap<String, Node>,
    pub indexes: Vec<String>,
    instances: u32,
    controls: Controls,
    process_start: Instant,
    callback: Option<Callback>,
    cache: HashMap<String, Vec<String>>,
    iter_count: usize
}

impl Hson {
    /// Create a new hson
    pub fn new () -> Hson {
        let hson = Hson {
            data: Vec::new(),
            nodes: HashMap::new(),
            indexes: Vec::new(),
            instances: 0,
            controls: Controls {
                chars: [OPEN_CURLY, CLOSE_CURLY, OPEN_ARR, CLOSE_ARR, COLONS, DOUBLE_QUOTES, COMMA],
                curly_brackets: 0,
                square_brackets: 0,
                double_quotes: 0
            },
            process_start: Instant::now(),
            callback: None,
            cache: HashMap::new(),
            iter_count: 0
        };

        hson
    }

    /// Create a new hson starting instances count with the provided number
    pub fn new_slice (instances: u32) -> Hson {
        let hson = Hson {
            data: Vec::new(),
            nodes: HashMap::new(),
            indexes: Vec::new(),
            instances,
            controls: Controls {
                chars: [OPEN_CURLY, CLOSE_CURLY, OPEN_ARR, CLOSE_ARR, COLONS, DOUBLE_QUOTES, COMMA],
                curly_brackets: 0,
                square_brackets: 0,
                double_quotes: 0
            },
            process_start: Instant::now(),
            callback: None,
            cache: HashMap::new(),
            iter_count: 0
        };

        hson
    }

    /// Parse an hson string
    pub fn parse (&mut self, data_to_parse: &str) -> Result<(), Error> {
        let data: Vec<char> = self.clean(&data_to_parse);
        let mut previous = ' ';
        let mut in_string = false;
        let mut string_just_closed = false;
        let mut skip = false;
        let l = data.len();
        let mut i = 0;
        let mut root_uid = String::from("");

        loop {
            let c = data[i];

            // If structure does not start with curly bracket throw error
            if i == 0 && c != OPEN_CURLY {
                let e = Error::new(ErrorKind::InvalidData, "Invalid character at 0");
                return Err(e);
            }

            in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != '\\';
            string_just_closed = self.controls.double_quotes > 0 && c == DOUBLE_QUOTES && previous != '\\';

//            println!("CHAR: {}", &c);
//            println!("IN_STRING: {}", &in_string);
//            println!("STRING CLOSED: {}", &string_just_closed);

            if !in_string {
                match self.controls.chars.iter().position(|&s| s == c) {
                    Some(_) => {
                        self.controls_count(&c, &previous);

                        if skip {
                            skip = false;
                        }
                    },
                    None => {}
                }
            }

//            println!("SKIP {}", &skip);

            if !in_string && !skip {
                let kind = match c {
                    CLOSE_CURLY => Kind::Null,
                    CLOSE_ARR => Kind::Null,
                    COMMA => Kind::Null,
                    COLONS => Kind::Null,
                    _ => {
                        self.get_node_kind(i, &data)?
                    }
                };

//                println!("KIND {:?}", &kind);

                match &kind {
                    &Kind::Bool |
                    &Kind::Integer |
                    &Kind::Float => {
                        skip = true;
                    },
                    _ => {}
                }

                let insert = match &kind {
                    &Kind::Null => {
                        false
                    },
                    &Kind::String => {
                        if string_just_closed {
                            false
                        } else {
                            let is_before = self.is_before_colons(i, &data);
//                            println!("BEFORE COLONS {}", &is_before);

                            !is_before
                        }
                    },
                    _ => true
                };

//                println!("INSERT {}", &insert);

                if insert {
                    let uid = Uuid::new_v4().to_string();
                    let root = i == 0;
                    let parent = if root { String::from("") } else {
                        self.get_previous_opened_node(self.indexes.len(), true, &Kind::Null)?
                    };
                    let parent_is_array = self.node_is_array(&parent);

//                    println!("PARENT ARRAY {}", &parent_is_array);

                    let key = if root || parent_is_array { [0, 0] } else {
                        self.get_node_key_position(i, &data)?
                    };
                    let value = if root { [i, data.len()] } else {
                        match &kind {
                            &Kind::Bool |
                            &Kind::Integer |
                            &Kind::Float => [i, data.len()],
                            _ => [i + 1, data.len()]
                        }

                    };

                    if root {
                        root_uid = uid.clone();
                    }

                    // TODO: IMPROVE PERF
                    // Insert the new node
                    self.instances += 1;
                    self.indexes.push(uid.clone());
                    self.nodes.insert(uid.clone(), Node {
                        root,
                        kind: kind.clone(),
                        parent: parent.clone(),
                        childs: Vec::new(),
                        key,
                        value,
                        id: uid.clone(),
                        opened: true,
                        instance: self.instances
                    });

                    if !root {
                        match self.nodes.get_mut(&parent) {
                            Some(node) => node.childs.push(uid.clone()),
                            None => {}
                        }

                        // TODO: IMPROVE PERF
                        if key != [0, 0] {
                            let mut key_str = String::from("");
                            for e in key[0]..key[1] {
                                key_str.push(data[e]);
                            }
                            self.caching(key_str, uid.clone());
                        }
                    }
                }

                let close = match &kind {
                    &Kind::Bool |
                    &Kind::Integer |
                    &Kind::Float => true,
                    _ => {
                        match c {
                            CLOSE_CURLY => true,
                            CLOSE_ARR => true,
                            DOUBLE_QUOTES => {
                                if string_just_closed {
                                    let is_before = self.is_before_colons(i, &data);
//                                    println!("BEFORE COLONS {}", &is_before);

                                    if is_before { false } else { true }
                                } else { false }
                            },
                            _ => false
                        }
                    }
                };

//                println!("CLOSE {}", &close);

                if close {
                    match &kind {
                        &Kind::Bool |
                        &Kind::Integer |
                        &Kind::Float => {
                            let v = self.extract_value(i, &data)?;
                            let current_node_id = self.get_previous_opened_node(self.nodes.len(), false, &kind)?;

                            match self.nodes.get_mut(&current_node_id) {
                                Some(node) => {
                                    node.value[1] = i + v.len();
                                    node.opened = false;
                                },
                                None => {}
                            }
                        },
                        _ => {
                            let closing_kind = match c {
                                CLOSE_CURLY => Kind::Node,
                                CLOSE_ARR => Kind::Array,
                                DOUBLE_QUOTES => Kind::String,
                                _ => continue
                            };
                            let previous_node_id = self.get_previous_opened_node(self.nodes.len(), true, &closing_kind)?;

                            match self.nodes.get_mut(&previous_node_id) {
                                Some(node) => {
                                    node.value[1] = i;
                                    node.opened = false;
                                },
                                None => {}
                            }
                        }
                    }
                }
            }

            previous = c;
            i += 1;

            if i >= l {
                break;
            }
        }

        self.data = data;
        self.validate()?;

        match self.callback {
            Some(c) => c(Event::Parse, root_uid),
            None => {}
        }

        Ok(())
    }

    /// Stringify and return the hson
    pub fn stringify (&self) -> String {
        let s: String = self.data.iter().collect();

        s
    }

    /// Retrieve root node id
    pub fn get_root (&mut self) -> String {
        let uid = self.indexes[0].clone();
        uid
    }

    /// Same as `get_root` but return the node itself
    pub fn get_root_node (&mut self) -> Option<&Node> {
        let uid = self.get_root();
        self.nodes.get(&uid)
    }

    /// Retrieve a node key
    pub fn get_node_key (&self, node: &Node) -> String {
        let mut key = String::from("");
        let start = node.key[0];
        let end = node.key[1];

        for i in start..end {
            key.push(self.data[i]);
        }

        key
    }

    /// Retrieve a node value
    pub fn get_node_value (&self, node: &Node) -> String {
        let mut value = String::from("");
        let start = node.value[0];
        let end = node.value[1];

        for i in start..end {
            value.push(self.data[i]);
        }

        value
    }

    /// Get all childs of a node recursively
    pub fn get_all_childs (&self, node_id: &String) -> Result<Vec<String>, Error> {
        match self.nodes.get(node_id) {
            Some(node) => {
                let mut results = Vec::new();

                if node.childs.len() > 0 {
                    results.append(&mut node.childs.clone());

                    for uid in &node.childs {
                        let mut res = self.get_all_childs(uid)?;
                        results.append(&mut res);
                    }
                }

                Ok(results)
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Cannot find node id {}", node_id));
                return Err(e);
            }
        }
    }

    /// Same as `get_all_childs` but returning nodes structures instead of their ids
    pub fn get_all_node_childs (&self, node: &Node) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();

        if node.childs.len() > 0 {
            for uid in &node.childs {
                match self.nodes.get(uid) {
                    Some(n) => {
                        results.push(n);

                        let mut res = self.get_all_node_childs(n)?;
                        results.append(&mut res);
                    },
                    None => {}
                }
            }
        }

        Ok(results)
    }

    /// Is provided node a descendant of the provided parent
    pub fn is_descendant (&self, parent_id: &str, child_id: &str) -> bool {
        let mut current = child_id.to_string().clone();

        loop {
            match self.nodes.get(&current) {
                Some(node) => {
                    if node.parent == parent_id {
                        return true
                    } else {
                        if node.root {
                            return false
                        }

                        current = node.parent.clone();
                    }
                },
                None => return false
            }
        }
    }

    /// Subscribe to events
    pub fn subscribe (&mut self, callback: Callback) {
        self.callback = Some(callback);
    }

    /// Get node clone with its key and value
    pub fn get_vertex (&self, node_id: &str) -> Option<Vertex> {
        match self.nodes.get(node_id) {
            Some(node) => {
                let key = self.get_node_key(&node);
                let value = self.get_node_value(&node);

                Some(Vertex {
                    root: node.root,
                    kind: node.kind.clone(),
                    parent: node.parent.clone(),
                    childs: node.childs.clone(),
                    id: node.id.clone(),
                    instance: node.instance,
                    key,
                    value
                })
            },
            None => None
        }
    }


    /* PRIVATE */
    /// Retrieve a node key position
    fn get_node_key_position (&self, mut data_start_pos: usize, data: &Vec<char>) -> Result<[usize; 2], Error> {
        let mut k = [0, data_start_pos];
        let mut on_match: i8 = 1;

        loop {
            data_start_pos = if data_start_pos > 0 { data_start_pos - 1 } else {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid format at {}", data_start_pos));
                return Err(e);
            };

            if data[data_start_pos] == '"' {
                k[on_match as usize] = if on_match == 0 { data_start_pos + 1 } else { data_start_pos };
                on_match -= 1;
            }

            if on_match < 0 {
                break;
            }
        }

        Ok(k)
    }

    /// Retrieve a node type
    fn get_node_kind (&self, data_start_pos: usize, data: &Vec<char>) -> Result<Kind, Error> {
        match data[data_start_pos] {
            '{' => Ok(Kind::Node),
            '[' => Ok(Kind::Array),
            '"' => Ok(Kind::String),
            _ => {
                let mut k = Kind::String;
                let v = self.extract_value(data_start_pos, &data)?;

                if v.parse::<i64>().is_ok() {
                    k = Kind::Integer;
                } else if v.parse::<f64>().is_ok() {
                    k = Kind::Float;
                } else if v == "true" || v == "false" {
                    k = Kind::Bool;
                } else {
                    let e = Error::new(ErrorKind::InvalidData, format!("Invalid value {} at {}", v, data_start_pos));
                    return Err(e);
                }

                Ok(k)
            }
        }
    }

    // TODO: IMPROVE PERF
    /// Retrieve the previous opened node from the provided node position
    fn get_previous_opened_node (&self, node_start_pos: usize, skip_primitives: bool, kind: &Kind) -> Result<String, Error> {
        let nodes = &self.nodes;
        let mut prev_node_uid = "";
        let mut l = node_start_pos;

        loop {
            l = if l > 0 { l - 1 } else {
                let e = Error::new(ErrorKind::Other, format!("Cannot retrieve previous opened node"));
                return Err(e);
            };

            match nodes.get(&self.indexes[l]) {
                Some(n) => {
                    if n.opened {
                        if skip_primitives {
                            match n.kind {
                                Kind::Float |
                                Kind::Integer |
                                Kind::Bool => { continue },
                                _ => {
                                    if kind == &Kind::Null {
                                        prev_node_uid = &n.id;
                                        break;
                                    } else {
                                        if kind == &n.kind {
                                            prev_node_uid = &n.id;
                                            break;
                                        }
                                    }
                                }
                            }
                        } else {
                            if kind == &Kind::Null {
                                prev_node_uid = &n.id;
                                break;
                            } else {
                                if kind == &n.kind {
                                    prev_node_uid = &n.id;
                                    break;
                                }
                            }
                        }
                    }
                },
                None => {}
            };
        }

        Ok(prev_node_uid.to_string())
    }

    /// Guess if position is before colons or not. Must be used on opening double quotes
    fn is_before_colons (&self, mut data_start_pos: usize, data: &Vec<char>) -> bool {
        loop {
            data_start_pos += 1;

            if data_start_pos >= data.len() {
                break;
            }

            if data[data_start_pos] == COLONS && data[data_start_pos - 1] == DOUBLE_QUOTES { return true; }

            if data[data_start_pos] == DOUBLE_QUOTES && data[data_start_pos - 1] != '\\' {
                return data[data_start_pos + 1] == COLONS;
            }
        }

        false
    }

    /// Is the node an array
    fn node_is_array (&self, node_id: &str) -> bool {
        match self.nodes.get(node_id) {
            Some(node) => node.kind == Kind::Array,
            None => false
        }
    }

    /// Extract the value from a start position
    fn extract_value (&self, data_start_pos: usize, data: &Vec<char>) -> Result<String, Error> {
        let mut n = data_start_pos;

        loop {
            n += 1;
            match self.controls.chars.iter().position(|&s| s == data[n]) {
                Some(_) => {
                    let v: String = Vec::from_iter(data[data_start_pos..n].iter().cloned()).into_iter().collect();
                    return Ok(v);
                },
                None => {}
            }

            if n > data.len() {
                let e = Error::new(ErrorKind::InvalidData, format!("Cannot retrieve node value from position {}", data_start_pos));
                return Err(e);
            }
        }
    }

    /// Insert hson slice into data
    fn insert_into_data (&mut self, hson: Hson, start: usize) -> Hson {
        let mut i = start;
        let l = hson.data.len() - 2;

        for (j, c) in hson.data.iter().enumerate() {
            if j > 0 && j <= l {
                self.data.insert(i, c.clone());
                i += 1;
            }
        }

        hson
    }

    /// Insert hson slice nodes into existing nodes
    fn insert_into_nodes (&mut self, parent_id: String, start_idx: usize, mut insert_pos: usize, mut hson: Hson) -> Hson {
        let mut root_id = String::from("");

        for (_i, key) in hson.indexes.iter().enumerate() {
            match hson.nodes.remove_entry(key) {
                Some((k, mut node)) => {
                    if node.root {
                        root_id = node.id;
                    } else {
                        // Insert the node into its new parent node
                        if node.parent == root_id {
                            node.parent = parent_id.clone();
                            match self.nodes.get_mut(&parent_id) {
                                Some(n) => {
                                    n.childs.insert(insert_pos, node.id.clone());
                                    insert_pos += 1;
                                },
                                None => {}
                            }
                        }

                        // Replace key and value position
                        if node.key != [0, 0] {
                            node.key[0] += start_idx - 1;
                            node.key[1] += start_idx - 1;
                        }

                        node.value[0] += start_idx - 1;
                        node.value[1] += start_idx - 1;

                        let idx = node.instance as usize;
                        self.indexes.insert(idx - 1, key.clone());
                        self.nodes.insert(k, node);
                    }
                },
                None => {}
            }
        }

        hson
    }

    /// Insert hson slice nodes into existing cache
    fn insert_into_cache (&mut self, hson: Hson) -> Hson {
        let keys = hson.cache.keys();
        let mut hson_cache_copy = hson.cache.clone();

        for key in keys {
            match hson_cache_copy.get_mut(key) {
                Some(mut hson_cache) => {

                    match self.cache.get_mut(key) {
                        Some(self_cache) => {
                            self_cache.append(&mut hson_cache);

                            let nodes = self.nodes.clone();
                            self_cache.sort_by(|a, b| {
                                let first = nodes.get(a).unwrap();
                                let second = nodes.get(b).unwrap();

                                first.instance.cmp(&second.instance)
                            });
                        },
                        None => {
                            self.cache.insert(key.clone(), hson_cache.clone());
                        }
                    }
                },
                None => {}
            }
        }



        hson
    }

    /// Right push existing nodes instance, key and value
    fn right_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error> {
        let l = self.indexes.len();
        let mut i = 0;
        let root_id = self.get_root();

        match self.nodes.get_mut(&root_id) {
            Some(node) => node.value[1] += data_size,
            None => {}
        }

        loop {
            let key = &self.indexes[i];
            match self.nodes.get_mut(key) {
                Some(n) => {
                    if n.instance >= start {
                        n.instance += distance;
                        n.key[0] += data_size;
                        n.key[1] += data_size;
                        n.value[0] += data_size;
                        n.value[1] += data_size;
                    }
                },
                None => {}
            }

            i += 1;
            if i >= l {
                break;
            }
        }

        self.instances += distance;

        Ok(())
    }

    /// Remove a node from data
    fn remove_from_data (&mut self, begin: usize, end: usize) {
        self.data.splice(begin..end, vec!());
    }

    /// Remove a node from nodes
    fn remove_from_nodes (&mut self, parent_id: &str, uid: &str) {
        // Remove the node from its parent childs
        match self.nodes.get_mut(parent_id) {
            Some(n) => {
                match n.childs.iter().position(|s| s == uid) {
                    Some(i) => {
                        n.childs.remove(i);
                    },
                    None => {}
                }
            },
            None => {}
        };

        // Remove the node from the existing indexes
        match self.indexes.iter().position(|s| s == uid) {
            Some(i) => {
                self.indexes.remove(i);
            },
            None => {}
        };

        // Remove all node childs from the existing nodes
        match self.get_all_childs(&uid.to_string()) {
            Ok(childs) => {
                for child in childs {
                    self.nodes.remove(&child);
                    match self.indexes.iter().position(|s| s == &child) {
                        Some(i) => {
                            self.indexes.remove(i);
                        },
                        None => {}
                    };
                }
            },
            Err(_e) => {}
        };

        self.nodes.remove(uid);
    }

    /// Remove a node from the cache
    fn remove_from_cache (&mut self, key: &str, node_id: &str) {
        match self.cache.get_mut(key) {
            Some(v) => {
                let mut to_remove = -1;
                for (idx, id) in v.iter().enumerate() {
                    if id == node_id {
                        to_remove = idx as i32;
                        break;
                    }
                }

                v.remove(to_remove as usize);

                if v.len() == 0 {
                    self.cache.remove_entry(key);
                }
            },
            None => {}
        }
    }

    /// Left push existing nodes instance, key and value
    fn left_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error> {
        let l = self.indexes.len();
        let mut i = 0;
        let root_id = self.get_root();

        match self.nodes.get_mut(&root_id) {
            Some(node) => node.value[1] -= data_size,
            None => {}
        }

        loop {
            let key = &self.indexes[i];
            match self.nodes.get_mut(key) {
                Some(n) => {
                    if n.instance >= start {
                        n.instance -= distance;
                        n.key[0] -= data_size;
                        n.key[1] -= data_size;
                        n.value[0] -= data_size;
                        n.value[1] -= data_size;
                    }
                },
                None => {}
            }

            i += 1;
            if i >= l {
                break;
            }
        }

        self.instances -= distance;

        Ok(())
    }

    /// Insert a comma after the provided node and push right key and value positions
    fn insert_comma (&mut self, uid: String, parent_uid: String) {
        match self.nodes.get(&uid) {
            Some(node) => {
                let mut instance = node.instance;
                let pos = node.value[1] + 1;
                let mut i = 0;

                if node.childs.len() > 0 {
                    match self.nodes.get(&node.childs[node.childs.len() - 1]) {
                        Some(child) => {
                            instance = child.instance;
                        },
                        None => {}
                    }
                }

                self.data.insert(pos, ',');
                loop {
                    let idx = &self.indexes[i];
                    match self.nodes.get_mut(idx) {
                        Some(n) => {
                            if i == 0 {
                                n.value[1] += 1;
                            } else if n.instance > instance {
                                n.key[0] += 1;
                                n.key[1] += 1;
                                n.value[0] += 1;
                                n.value[1] += 1;
                            } else if n.id == parent_uid {
                                n.value[1] += 1;
                            }
                        },
                        None => {}
                    }

                    i += 1;
                    if i >= self.indexes.len() {
                        break;
                    }
                }
            },
            None => {}
        };
    }

    /// Recursive method looking for nodes matching the query
    fn retrieve (&mut self, query: Vec<&str>) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let mut tmp: Vec<(String, String)> = Vec::new();
        let mut i = (query.len() - 1) as i32;

        loop {
            if i as usize == query.len() - 1 {
                match self.cache.get(query[i as usize]) {
                    Some(v) => {
                        for uid in v {
                            match self.nodes.get(uid) {
                                Some(n) => {
                                    let n: (String, String) = (n.id.clone(), n.parent.clone());
                                    tmp.push(n);
                                },
                                None => {}
                            }
                        }
                    },
                    None => return Ok(results)
                }
            } else {
                let mut res: Vec<(String, String)> = Vec::new();

                for map in &tmp {
                    let parent = &map.1;

                    match self.nodes.get(parent) {
                        Some(node) => {
                            let key = self.get_node_key(node);

                            if key == query[i as usize] {
                                let n: (String, String) = (map.0.clone(), node.parent.clone());
                                res.push(n);
                            }
                        },
                        None => {}
                    }
                }

                tmp = res;
            }

            i -= 1;
            if i < 0 {
                break;
            }
        }

        for t in tmp {
            results.push(t.0);
        }

        Ok(results)
    }

    /// Clean a string of tab/newlines/spaces
    fn clean (&self, s: &str) -> Vec<char> {
        let mut string_array = Vec::new();
        let mut in_string = false;
        let mut previous = ' ';

        for (_i, c) in s.chars().enumerate() {
            if c == '"' {
                if !in_string {
                    in_string = true;
                } else {
                    if previous != '\\' {
                        in_string = false;
                    }
                }
            }

            if in_string || (c != ' ' && c != '\t' && c != '\r' && c != '\n') {
                string_array.push(c);
            }

            previous = c;
        }

        string_array
    }

    /// Cache
    fn caching (&mut self, key: String, uid: String) {
        match self.cache.get_mut(&key) {
            Some(v) => v.push(uid),
            None => {
                let mut ids = Vec::new();
                ids.push(uid);

                self.cache.insert(key, ids);
            }
        }
    }

    /// Deduplicate vector's values
    fn unique (&self, v: &Vec<String>) -> Vec<String> {
        let mut results = Vec::new();

        for value in v {
            if !results.contains(value) {
                results.push(value.clone());
            }
        }

        results
    }

    fn controls_count (&mut self, c: &char, previous: &char) {
        if c == &OPEN_CURLY {
            self.controls.curly_brackets += 1;
        }

            else if c == &CLOSE_CURLY {
                self.controls.curly_brackets -= 1;
            }

                else if c == &DOUBLE_QUOTES {
                    if self.controls.double_quotes > 0 && previous != &'\\' {
                        self.controls.double_quotes = 0;
                    } else {
                        self.controls.double_quotes = 1;
                    }
                }

                    else if c == &OPEN_ARR {
                        self.controls.square_brackets += 1;
                    }

                        else if c == &CLOSE_ARR {
                            self.controls.square_brackets -= 1;
                        }
    }

    fn validate (&self) -> Result<(), Error> {
        for (_key, value) in &self.nodes {
            if value.opened {
                let mut key = self.get_node_key(value);

                if key.is_empty() {
                    key = String::from("root");
                }

                let e = Error::new(ErrorKind::InvalidData, format!("Invalid instance `{}`", value.instance));
                return Err(e);
            }
        }

        Ok(())
    }
}


pub trait Query {
    fn query (&mut self, q: &str) -> Result<Vec<String>, Error>;

    fn query_nodes (&mut self, q: &str) -> Result<Vec<&Node>, Error>;

    fn query_on (&mut self, node_id: &str, q: &str, recursive: bool) -> Result<Vec<String>, Error>;

    fn query_on_nodes (&mut self, node: &Node, q: &str, recursive: bool) -> Result<Vec<&Node>, Error>;
}

impl Query for Hson {
    /// Public method to query the data
    fn query (&mut self, q: &str) -> Result<Vec<String>, Error> {
        let parts: Vec<&str> = q.split(" ").collect();
        let results = self.retrieve(parts)?;

        Ok(results)
    }

    /// Same as `query` but return nodes structures instead of their ids
    fn query_nodes (&mut self, q: &str) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(" ").collect();

        let ids = self.retrieve(parts)?;
        for uid in &ids {
            results.push(&self.nodes[uid]);
        }

        Ok(results)
    }

    /// Same as `query` but constrain the search in the provided node's childs only
    fn query_on (&mut self, node_id: &str, q: &str, recursive: bool) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(" ").collect();

        let ids = self.retrieve(parts)?;
        for uid in &ids {
            if recursive {
                if self.is_descendant(node_id, uid) {
                    results.push(uid.clone());
                }
            } else {
                match self.nodes.get(uid) {
                    Some(n) => {
                        if n.parent == node_id {
                            results.push(uid.clone());
                        }
                    },
                    None => {}
                }
            }
        }

        Ok(results)
    }

    /// Same as `query_on` but return nodes structures instead of their ids
    fn query_on_nodes (&mut self, node: &Node, q: &str, recursive: bool) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(" ").collect();

        let ids = self.retrieve(parts)?;
        for uid in &ids {
            if recursive {
                if self.is_descendant(&node.id, uid) {
                    results.push(&self.nodes[uid]);
                }
            } else {
                match self.nodes.get(uid) {
                    Some(n) => {
                        if n.parent == node.id {
                            results.push(&self.nodes[uid]);
                        }
                    },
                    None => {}
                }
            }
        }

        Ok(results)
    }
}


pub trait Ops {
    fn insert (&mut self, uid: &String, idx: usize, s: &str) -> Result<(), Error>;

    fn remove (&mut self, uid: &String) -> Result<(), Error>;
}

impl Ops for Hson {
    /// Insert an hson slice
    fn insert (&mut self, node_id: &String, insert_pos: usize, data_to_insert: &str) -> Result<(), Error> {
        let mut slice_range = 0;

        match self.nodes.get(node_id) {
            Some(node) => {
                let mut t = self.clean(&data_to_insert);
                // Start instances count (for new_slice method) at the provided node instance number
                // Subtract 1 to take care of the root instance in the new hson slice
                let mut start_instance = node.instance - 1;
                // From what instance the existing nodes should be pushed to the right
                let mut start = node.instance + 1;
                // From what position should the new slice should be inserted in the existing data
                let mut start_idx = node.value[0];
                let parent_id = node.id.clone();

                // If the inserting position in the node's childs is not the first one,
                // retrieve the new position based on the child values
                if insert_pos > 0 {
                    let child_uid = match node.childs.get(insert_pos - 1) {
                        Some(id) => id,
                        None => {
                            let e = Error::new(ErrorKind::InvalidData, format!("Invalid index {}", insert_pos));
                            return Err(e);
                        }
                    };
                    let child = match self.nodes.get(child_uid) {
                        Some(c) => {
                            if c.childs.len() > 0 {
                                match self.nodes.get(&c.childs[c.childs.len() - 1]) {
                                    Some(sc) => sc,
                                    None => c
                                }
                            } else {
                                c
                            }
                        },
                        None => {
                            let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", child_uid));
                            return Err(e);
                        }
                    };

                    start = child.instance + 1;
                    start_instance = child.instance - 1;
                    // Add 2 to take care of comma char
                    start_idx = child.value[1] + 2;
                }

                if node.childs.len() > 0 {
                    // Insert a comma if the inserting position is not the last one and there's not already one
                    // When inserting in the middle of childs add the comma and push the position of all following nodes
                    if insert_pos < node.childs.len() && t[t.len() - 2] != COMMA {
                        t.insert(t.len() - 1, ',');
                    } else if insert_pos >= node.childs.len() {
                        let last_child_uid = node.childs[node.childs.len() - 1].clone();
                        let current_uid = node.id.clone();
                        self.insert_comma(last_child_uid, current_uid);
                        start_idx += 1;
                    }
                }

                // Parsing the new slice occurs only here to allow borrowing as there's no more use of the node variable
                let s: String = t.into_iter().collect();
                let s = s.as_str();
                let mut hson = Hson::new_slice(start_instance);
                hson.parse(s)?;

                let root_id = hson.get_root();
                match hson.nodes.get(&root_id) {
                    Some(n) => {
                        slice_range = n.value[1] - n.value[0];
                    },
                    None => {}
                }

                let num_keys = hson.nodes.keys().len() as u32;
                // How much does the existing nodes must be pushed
                let distance = num_keys - 1;
                // Data size subtracting the root's curly bracket chars
                let data_size = hson.data.len() - 2;

                self.right_push_instances(start, distance, data_size)?;
                hson = self.insert_into_data(hson, start_idx);
                hson = self.insert_into_nodes(parent_id, start_idx, insert_pos,hson);
                hson = self.insert_into_cache(hson);
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", node_id));
                return Err(e);
            }
        };

        match self.nodes.get_mut(node_id) {
            Some(node) => {
                node.value[1] += slice_range;
            },
            None => {}
        };

        match self.callback {
            Some(c) => c(Event::Insert, node_id.clone()),
            None => {}
        }

        Ok(())
    }

    /// Remove a node and all its childs
    fn remove (&mut self, node_id: &String) -> Result<(), Error> {
        match self.nodes.get(node_id) {
            Some(node) => {
                let key = self.get_node_key(node);
                let childs = self.get_all_childs(node_id)?;
                let instances_range = childs.len() + 1;
                let start_instance = node.instance + childs.len() as u32 + 1;
                let parent_id = node.parent.clone();
                let data_start_pos = node.key[0];
                let mut data_end_pos = node.value[1] + 1;
                let mut data_size = node.value[1] - node.key[0];

                if self.data[data_end_pos] == COMMA {
                    data_end_pos += 1;
                    data_size += 1;
                }

                if !key.is_empty() {
                    self.remove_from_cache(&key, node_id);
                }

                for child in childs {
                    match self.nodes.get(&child) {
                        Some(n) => {
                            let key = self.get_node_key(n);

                            if !key.is_empty() {
                                self.remove_from_cache(&key, &child);
                            }
                        },
                        None => {}
                    }
                }

                self.left_push_instances(start_instance, instances_range as u32, data_size)?;
                self.remove_from_data(data_start_pos, data_end_pos);
                self.remove_from_nodes(&parent_id, node_id);
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", node_id));
                return Err(e);
            }
        }

        match self.callback {
            Some(c) => c(Event::Remove, node_id.clone()),
            None => {}
        }

        Ok(())
    }
}


pub trait Cast {
    fn key_as_string (&self) -> Option<String>;

    fn key_as_f64 (&self) -> Option<f64>;

    fn key_as_i64 (&self) -> Option<i64>;

    fn key_as_u64 (&self) -> Option<u64>;

    fn key_as_bool (&self) -> Option<bool>;

    fn value_as_string (&self) -> Option<String>;

    fn value_as_f64 (&self) -> Option<f64>;

    fn value_as_i64 (&self) -> Option<i64>;

    fn value_as_u64 (&self) -> Option<u64>;

    fn value_as_bool (&self) -> Option<bool>;

    fn value_as_array (&self) -> Option<Vec<String>>;

    fn as_f64 (&self, value: &str) -> Option<f64>;

    fn as_i64 (&self, value: &str) -> Option<i64>;

    fn as_u64 (&self, value: &str) -> Option<u64>;

    fn as_bool (&self, value: &str) -> Option<bool>;
}

impl Cast for Vertex {
    fn key_as_string (&self) -> Option<String> {
        let v = self.key.clone();
        Some(v)
    }

    fn key_as_f64 (&self) -> Option<f64> {
        self.as_f64(&self.key)
    }

    fn key_as_i64 (&self) -> Option<i64> {
        self.as_i64(&self.key)
    }

    fn key_as_u64 (&self) -> Option<u64> {
        self.as_u64(&self.key)
    }

    fn key_as_bool (&self) -> Option<bool> {
        self.as_bool(&self.key)
    }

    fn value_as_string (&self) -> Option<String> {
        let v = self.value.clone();
        Some(v)
    }

    fn value_as_f64 (&self) -> Option<f64> {
        self.as_f64(&self.value)
    }

    fn value_as_i64 (&self) -> Option<i64> {
        self.as_i64(&self.value)
    }

    fn value_as_u64 (&self) -> Option<u64> {
        self.as_u64(&self.value)
    }

    fn value_as_bool (&self) -> Option<bool> {
        self.as_bool(&self.value)
    }

    fn value_as_array (&self) -> Option<Vec<String>> {
        let chars: Vec<char> = self.value.chars().collect();
        let mut values: Vec<String> = Vec::new();
        let mut in_string = false;
        let mut previous = &' ';
        let mut item = String::from("");

        for c in &chars {
            if c == &'"' {
                if !in_string {
                    in_string = true;
                } else {
                    if previous != &'\\' {
                        in_string = false;
                    }
                }
            } else if c == &',' && !in_string {
                values.push(item);
                item = String::from("");
            } else {
                item.push_str(&c.to_string());
            }

            previous = c;
        }

        if !item.is_empty() {
            values.push(item);
        }

        Some(values)
    }

    fn as_f64 (&self, value: &str) -> Option<f64> {
        let v = value.parse::<f64>();

        if v.is_ok() {
            Some(v.unwrap())
        } else {
            None
        }
    }

    fn as_i64 (&self, value: &str) -> Option<i64> {
        let v = value.parse::<i64>();

        if v.is_ok() {
            Some(v.unwrap())
        } else {
            None
        }
    }

    fn as_u64 (&self, value: &str) -> Option<u64> {
        let v = value.parse::<u64>();

        if v.is_ok() {
            Some(v.unwrap())
        } else {
            None
        }
    }

    fn as_bool (&self, value: &str) -> Option<bool> {
        let v = match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None
        };

        v
    }
}


pub trait Debug {
    fn print_nodes (&self, sorted: bool);

    fn print_data (&mut self, pretty: bool);

    fn print_indexes (&self);

    fn print_process_time (&self);

    fn print_controls (&self);

    fn print_cache (&self);
}

impl Debug for Hson {
    fn print_nodes (&self, sorted: bool) {
        if sorted {
            let mut previous_instance = self.instances - self.indexes.len() as u32;

            loop {
                for (key, value) in &self.nodes {
                    let node = self.nodes.get(key).unwrap();

                    if node.instance == previous_instance + 1 {
                        println!("{} : {:?}", self.get_node_key(value), value);
                        previous_instance += 1;
                    }
                }

                if previous_instance >= self.instances as u32 {
                    break;
                }
            }
        } else {
            for (_key, value) in &self.nodes {
                println!("{} : {:?}", self.get_node_key(value), value);
            }
        }
    }

    fn print_data (&mut self, pretty: bool) {
        if !pretty {
            let s: String = self.data.iter().collect();
            println!("{}", &s);
        } else {
            let mut i = 0;
            let previous = ' ';
            let mut indent = 0;
            let l = self.data.len() - 1;
            let mut in_array = false;

            loop {
                let c = self.data[i];
                self.controls_count(&c, &previous);
                let in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != '\\';

                if !in_string {
                    match self.controls.chars.iter().position(|&s| s == c) {
                        Some(_) => {
                            match c {
                                OPEN_CURLY => {
                                    in_array = false;
                                    print!("{}", c);
                                    indent += 1;
                                    print!("\n");
                                    for _t in 0..indent {
                                        print!("\t");
                                    }
                                },
                                CLOSE_CURLY => {
                                    indent -= 1;
                                    print!("\n");
                                    for _t in 0..indent {
                                        print!("\t");
                                    }
                                    print!("{}", c);
                                },
                                OPEN_ARR => {
                                    in_array = true;
                                    print!("{}", c);
                                },
                                CLOSE_ARR => {
                                    in_array = false;
                                    print!("{}", c);
                                },
                                COMMA => {
                                    print!("{}", c);
                                    if !in_array {
                                        print!("\n");
                                        for _t in 0..indent {
                                            print!("\t");
                                        }
                                    }
                                }
                                _ => {
                                    print!("{}", c);
                                }
                            }
                        },
                        None => {
                            print!("{}", c);
                        }
                    }
                } else {
                    print!("{}", c);
                }

                i += 1;
                if i > l {
                    break;
                }
            }
        }
    }

    fn print_indexes (&self) {
        for idx in &self.indexes {
            println!("{}", idx);
        }
    }

    fn print_process_time (&self) {
        let duration = self.process_start.elapsed();
        println!("{:?}", duration);
    }

    fn print_controls (&self) {
        println!("CURLY: {}\nSQUARE: {}\nQUOTES: {}",
                 self.controls.curly_brackets,
                 self.controls.square_brackets,
                 self.controls.double_quotes);
    }

    fn print_cache (&self) {
        let keys = self.cache.keys();

        for key in keys {
            println!("{}", &key);
            match self.cache.get(key) {
                Some(v) => {
                    for uid in v {
                        println!("\t{}", &uid);
                    }
                },
                None => {}
            }
        }
    }
}


pub trait Search {
    fn search (&mut self, query: &str) -> Result<Vec<String>, Error>;

    fn search_in (&mut self, node_id: &str, query: &str) -> Result<Vec<String>, Error>;
}

impl Search for Hson {
    /// Enhanced queries
    // standard search: div p
    // no recursive search: div>p
    // multiple search: div p|ul|article
    // equality search: div p attrs id='12'
    fn search (&mut self, query: &str) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let q = self.format_query(query);
        let root_id = self.get_root();
        let first = true;

        // Add the root node in the results list for first lookup
        results.push(root_id);
        for part in q {
            if part.contains(">") {
                results = self.find_childs(&part, &results, first)?;
            } else if part.contains("|") {
                results = self.find_multiple_childs(&part, &results)?;
            } else {
                results = self.find_descendants(&part, &results)?;
            }
        }

        // TODO: IMPROVE SEARCH TO AVOID DUPLICATE ENTRIES
        results = self.unique(&results);

        Ok(results)
    }

    fn search_in (&mut self, node_id: &str, query: &str) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let q = self.format_query(query);
        let first = true;

        // Add the root node in the results list for first lookup
        results.push(node_id.to_string());
        for part in q {
            if part.contains(">") {
                results = self.find_childs(&part, &results, first)?;
            } else if part.contains("|") {
                results = self.find_multiple_childs(&part, &results)?;
            } else {
                results = self.find_descendants(&part, &results)?;
            }
        }

        // TODO: IMPROVE SEARCH TO AVOID DUPLICATE ENTRIES
        results = self.unique(&results);

        Ok(results)
    }
}

trait SearchUtils {
    fn format_query (&self, query: &str) -> Vec<String>;

    fn clean_query (&self, query: &str) -> Vec<char>;

    fn find_descendants (&mut self, query: &str, existing: &Vec<String>) -> Result<Vec<String>, Error>;

    fn find_childs (&mut self, query: &str, existing: &Vec<String>, first: bool) -> Result<Vec<String>, Error>;

    fn find_multiple_childs (&mut self, query: &str, existing: &Vec<String>) -> Result<Vec<String>, Error>;

    fn filter_equality_childs (&mut self, query: &str, results: &Vec<String>) -> Result<Vec<String>, Error>;
}

impl SearchUtils for Hson {
    fn format_query (&self, query: &str) -> Vec<String> {
        let mut result = Vec::new();
        let q = self.clean_query(query);
        let mut in_string = false;
        let mut previous = ' ';
        let mut item = String::from("");

        for c in q {
            if c == '\'' {
                if !in_string {
                    in_string = true;
                } else {
                    if previous != '\\' {
                        in_string = false;
                    }
                }
            }

            if c == ' ' && !in_string {
                result.push(item);
                item = String::from("");
            } else {
                item.push_str(&c.to_string());
            }

            previous = c;
        }

        if !item.is_empty() {
            result.push(item);
        }

        result
    }

    fn clean_query (&self, query: &str) -> Vec<char> {
        let mut string_array = Vec::new();
        let mut in_string = false;
        let mut previous = ' ';

        for (_i, c) in query.chars().enumerate() {
            if c == '\'' {
                if !in_string {
                    in_string = true;
                } else {
                    if previous != '\\' {
                        in_string = false;
                    }
                }
            }

            if in_string {
                string_array.push(c);
            } else if c == '>' || c == '=' || c == '|' {
                let l = string_array.len();
                if string_array[l - 1] == ' ' {
                    string_array.remove(l - 1);
                }

                string_array.push(c);
            } else if c != ' ' && c != '\t' && c != '\r' && c != '\n' {
                string_array.push(c);
            } else {
                match previous {
                    ' ' |
                    '\t' |
                    '\r' |
                    '\n' |
                    '>' |
                    '=' |
                    '|' => continue,
                    _ => string_array.push(' ')
                }
            }

            previous = c;
        }

        string_array
    }

    fn find_descendants (&mut self, query: &str, existing: &Vec<String>) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();

        for r in existing {
            if query.contains("=") {
                let parts: Vec<&str> = query.split("=").collect();
                let mut t = self.query_on(r, parts[0], true)?;
                t = self.filter_equality_childs(query, &t)?;
                results.append(&mut t);
            } else {
                let mut t = self.query_on(r, query, true)?;
                results.append(&mut t);
            }
        }

        Ok(results)
    }

    fn find_childs (&mut self, query: &str, existing: &Vec<String>, mut first: bool) -> Result<Vec<String>, Error> {
        let mut results = existing.clone();
        let elements: Vec<&str> = query.split(">").collect();

        // Loop on each element of the query
        for elm in elements {
            let mut res = Vec::new();

            // And look for those query elements in existing results
            for r in &results {
                if elm.contains("=") {
                    let parts: Vec<&str> = elm.split("=").collect();
                    let mut t = self.query_on(r, parts[0], first)?;
                    t = self.filter_equality_childs(elm, &t)?;
                    res.append(&mut t);
                } else {
                    let mut t = self.query_on(r, elm, first)?;
                    res.append(&mut t);
                }
            }

            results = res;

            // First lookup must be recursive, following must not
            first = false;
        }

        Ok(results)
    }

    fn find_multiple_childs (&mut self, query: &str, existing: &Vec<String>) -> Result<Vec<String>, Error> {
        let elements: Vec<&str> = query.split("|").collect();
        let mut results = Vec::new();

        for elm in elements {
            for r in existing {
                if elm.contains("=") {
                    let parts: Vec<&str> = elm.split("=").collect();
                    let mut t = self.query_on(r, parts[0], true)?;
                    t = self.filter_equality_childs(elm, &t)?;
                    results.append(&mut t);
                } else {
                    let mut t = self.query_on(r, elm, true)?;
                    results.append(&mut t);
                }
            }
        }

        Ok(results)
    }

    fn filter_equality_childs (&mut self, query: &str, existing: &Vec<String>) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = query.split("=").collect();
        let chars: Vec<char> = parts[1].chars().collect();
        let equality = chars[1..chars.len()-1].iter().cloned().collect::<String>();
        let mut patterns = Vec::new();

        patterns.push(equality.as_str());
        if patterns[0].contains("|") {
            patterns = patterns[0].split("|").collect();
        }

        for res in existing {
            match self.nodes.get(res) {
                Some(node) => {
                    for pattern in &patterns {
                        let value = self.get_node_value(node);

                        if value == pattern.trim() {
                            results.push(res.clone());
                        }
                    }
                },
                None => {}
            }
        }

        Ok(results)
    }
}


impl Iterator for Hson {
    type Item = String;

    fn next (&mut self) -> Option<String> {
        if !self.indexes.is_empty() && self.iter_count < self.indexes.len() {
            let id = self.indexes[self.iter_count].clone();
            self.iter_count += 1;
            Some(id)
        } else {
            self.iter_count = 0;
            None
        }
    }
}
