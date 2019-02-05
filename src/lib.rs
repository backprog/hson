#![allow(unused_assignments)]

use std::collections::HashMap;
use std::vec::Vec;
use std::iter::FromIterator;
use std::io::{ ErrorKind, Error };
use std::time::{ Instant };


type Callback = fn(Event, u64);

const OPEN_CURLY: char = '{';
const CLOSE_CURLY: char = '}';
const OPEN_ARR: char = '[';
const CLOSE_ARR: char = ']';
const QUOTE: char = '\'';
const DOUBLE_QUOTES: char = '"';
const COLONS: char = ':';
const COMMA: char = ',';
const BACKSLASH: char = '\\';

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
    pub parent: u64,
    pub childs: Vec<u64>,
    pub key: [usize; 2],
    pub value: [usize; 2],
    pub id: u64,
    pub opened: bool,
    pub instance: u64
}

/// Hson cloned node
#[derive(Clone, Debug)]
pub struct Vertex {
    pub root: bool,
    pub kind: Kind,
    pub parent: u64,
    pub childs: Vec<u64>,
    pub id: u64,
    pub instance: u64,
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
    pub nodes: HashMap<u64, Node>,
    pub indexes: Vec<u64>,
    instances: u64,
    controls: Controls,
    process_start: Instant,
    callback: Option<Callback>,
    cache: HashMap<String, Vec<u64>>,
    id_count: u64,
    iter_count: usize
}

impl Hson {
    /// Create a new hson
    pub fn new () -> Hson {
        Hson {
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
            id_count: 0,
            iter_count: 0
        }
    }

    /// Create a new hson starting instances count with the provided number
    pub fn new_slice (start_id: u64, start_instance: u64) -> Hson {
        Hson {
            data: Vec::new(),
            nodes: HashMap::new(),
            indexes: Vec::new(),
            instances: start_instance,
            controls: Controls {
                chars: [OPEN_CURLY, CLOSE_CURLY, OPEN_ARR, CLOSE_ARR, COLONS, DOUBLE_QUOTES, COMMA],
                curly_brackets: 0,
                square_brackets: 0,
                double_quotes: 0
            },
            process_start: Instant::now(),
            callback: None,
            cache: HashMap::new(),
            id_count: start_id,
            iter_count: 0
        }
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

        if l > 0 {
            // If structure does not start with curly bracket throw error
            if data[0] != OPEN_CURLY {
                let e = Error::new(ErrorKind::InvalidData, "Invalid character at 0");
                return Err(e);
            }

            loop {
                let c = data[i];

                in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != BACKSLASH;
                string_just_closed = self.controls.double_quotes > 0 && c == DOUBLE_QUOTES && previous != BACKSLASH;

//            println!("CHAR: {}", &c);
//            println!("IN_STRING: {}", &in_string);
//            println!("STRING CLOSED: {}", &string_just_closed);

                if !in_string && self.controls.chars.iter().any(|&s| s == c) {
                    self.controls_count(c, previous);

                    if skip {
                        skip = false;
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

                    match kind {
                        Kind::Bool |
                        Kind::Integer |
                        Kind::Float => {
                            skip = true;
                        },
                        _ => {}
                    }

                    let insert = match kind {
                        Kind::Null => {
                            false
                        },
                        Kind::String => {
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
                        let root = i == 0;
                        let parent = if root { 0 } else {
                            self.get_previous_opened_node(self.indexes.len(), true, &Kind::Null)?
                        };
                        let parent_is_array = self.node_is_array(parent);

//                    println!("PARENT ARRAY {}", &parent_is_array);

                        let key = if root || parent_is_array { [0, 0] } else {
                            self.get_node_key_position(i, &data)?
                        };
                        let value = if root { [i, data.len()] } else {
                            match kind {
                                Kind::Bool |
                                Kind::Integer |
                                Kind::Float => [i, data.len()],
                                _ => [i + 1, data.len()]
                            }

                        };

                        // Insert the new node
                        self.id_count += 1;
                        self.instances += 1;
                        self.indexes.push(self.id_count);
                        self.nodes.insert(self.id_count, Node {
                            root,
                            kind: kind.clone(),
                            parent,
                            childs: Vec::new(),
                            key,
                            value,
                            id: self.id_count,
                            opened: true,
                            instance: self.instances
                        });

                        if !root {
                            if let Some(node) = self.nodes.get_mut(&parent) {
                                node.childs.push(self.id_count);
                            }

                            // TODO: IMPROVE PERF
                            if key != [0, 0] {
                                let mut key_str = String::from("");
                                for e in data.iter().take(key[1]).skip(key[0]) {
                                    key_str.push(e.clone());
                                }
                                self.caching(key_str, self.id_count);
                            }
                        }
                    }

                    let close = match kind {
                        Kind::Bool |
                        Kind::Integer |
                        Kind::Float => true,
                        _ => {
                            match c {
                                CLOSE_CURLY => true,
                                CLOSE_ARR => true,
                                DOUBLE_QUOTES => {
                                    if string_just_closed {
                                        let is_before = self.is_before_colons(i, &data);
//                                    println!("BEFORE COLONS {}", &is_before);

                                        !is_before
                                    } else { false }
                                },
                                _ => false
                            }
                        }
                    };

//                println!("CLOSE {}", &close);

                    if close {
                        match kind {
                            Kind::Bool |
                            Kind::Integer |
                            Kind::Float => {
                                let v = self.extract_value(i, &data)?;
                                let current_node_id = self.get_previous_opened_node(self.nodes.len(), false, &kind)?;

                                if let Some(node) = self.nodes.get_mut(&current_node_id) {
                                    node.value[1] = i + v.len();
                                    node.opened = false;
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

                                if let Some(node) = self.nodes.get_mut(&previous_node_id) {
                                    node.value[1] = i;
                                    node.opened = false;
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

            if let Some(c) = self.callback {
                c(Event::Parse, self.id_count);
            }
        }

        Ok(())
    }

    /// Stringify and return the hson
    pub fn stringify (&self) -> String {
        let s: String = self.data.iter().collect();

        s
    }

    /// Retrieve root node id
    pub fn get_root (&mut self) -> u64 {
        self.indexes[0]
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
    pub fn get_all_childs (&self, node_id: u64) -> Result<Vec<u64>, Error> {
        match self.nodes.get(&node_id) {
            Some(node) => {
                let mut results = Vec::new();

                if !node.childs.is_empty() {
                    results.append(&mut node.childs.clone());

                    for uid in &node.childs {
                        let mut res = self.get_all_childs(*uid)?;
                        results.append(&mut res);
                    }
                }

                Ok(results)
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Cannot find node id {}", node_id));
                Err(e)
            }
        }
    }

    /// Same as `get_all_childs` but returning nodes structures instead of their ids
    pub fn get_all_node_childs (&self, node: &Node) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();

        if !node.childs.is_empty() {
            for uid in &node.childs {
                if let Some(n) = self.nodes.get(uid) {
                    results.push(n);

                    let mut res = self.get_all_node_childs(n)?;
                    results.append(&mut res);
                }
            }
        }

        Ok(results)
    }

    /// Is provided node a descendant of the provided parent
    pub fn is_descendant (&self, parent_id: u64, child_id: u64) -> bool {
        let mut current = child_id;

        loop {
            match self.nodes.get(&current) {
                Some(node) => {
                    if node.parent == parent_id {
                        return true
                    } else {
                        if node.root {
                            return false
                        }

                        current = node.parent;
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
    pub fn get_vertex (&self, node_id: u64) -> Option<Vertex> {
        match self.nodes.get(&node_id) {
            Some(node) => {
                let key = self.get_node_key(&node);
                let value = self.get_node_value(&node);

                Some(Vertex {
                    root: node.root,
                    kind: node.kind.clone(),
                    parent: node.parent,
                    childs: node.childs.clone(),
                    id: node.id,
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
    fn get_node_key_position (&self, mut data_start_pos: usize, data: &[char]) -> Result<[usize; 2], Error> {
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
    fn get_node_kind (&self, data_start_pos: usize, data: &[char]) -> Result<Kind, Error> {
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
    fn get_previous_opened_node (&self, node_start_pos: usize, skip_primitives: bool, kind: &Kind) -> Result<u64, Error> {
        let nodes = &self.nodes;
        let mut prev_node_uid = 0;
        let mut l = node_start_pos;

        loop {
            l = if l > 0 { l - 1 } else {
                let e = Error::new(ErrorKind::Other, "Cannot retrieve previous opened node");
                return Err(e);
            };

            if let Some(n) = nodes.get(&self.indexes[l]) {
                if n.opened {
                    if skip_primitives {
                        match n.kind {
                            Kind::Float |
                            Kind::Integer |
                            Kind::Bool => { continue },
                            _ => {
                                if kind == &Kind::Null || kind == &n.kind {
                                    prev_node_uid = n.id;
                                    break;
                                }
                            }
                        }
                    } else if kind == &Kind::Null || kind == &n.kind {
                        prev_node_uid = n.id;
                        break;
                    }
                }
            };
        }

        Ok(prev_node_uid)
    }

    /// Guess if position is before colons or not. Must be used on opening double quotes
    fn is_before_colons (&self, mut data_start_pos: usize, data: &[char]) -> bool {
        loop {
            data_start_pos += 1;

            if data_start_pos >= data.len() {
                break;
            }

            if data[data_start_pos] == COLONS && data[data_start_pos - 1] == DOUBLE_QUOTES { return true; }

            if data[data_start_pos] == DOUBLE_QUOTES && data[data_start_pos - 1] != BACKSLASH {
                return data[data_start_pos + 1] == COLONS;
            }
        }

        false
    }

    /// Is the node an array
    fn node_is_array (&self, node_id: u64) -> bool {
        match self.nodes.get(&node_id) {
            Some(node) => node.kind == Kind::Array,
            None => false
        }
    }

    /// Extract the value from a start position
    fn extract_value (&self, data_start_pos: usize, data: &[char]) -> Result<String, Error> {
        let mut n = data_start_pos;

        loop {
            n += 1;
            if self.controls.chars.iter().any(|&s| s == data[n]) {
                let v: String = Vec::from_iter(data[data_start_pos..n].iter().cloned()).into_iter().collect();
                return Ok(v);
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
    fn insert_into_nodes (&mut self, parent_id: u64, start_idx: usize, mut insert_pos: usize, mut hson: Hson) -> Hson {
        let mut root_id = 0;

        for (_i, key) in hson.indexes.iter().enumerate() {
            if let Some((k, mut node)) = hson.nodes.remove_entry(key) {
                if node.root {
                    root_id = node.id;
                } else {
                    // Insert the node into its new parent node
                    if node.parent == root_id {
                        node.parent = parent_id;
                        if let Some(n) = self.nodes.get_mut(&parent_id) {
                            n.childs.insert(insert_pos, node.id);
                            insert_pos += 1;
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
                    self.indexes.insert(idx - 1, *key);
                    self.nodes.insert(k, node);
                }
            }
        }

        hson
    }

    /// Insert hson slice nodes into existing cache
    fn insert_into_cache (&mut self, hson: Hson) -> Hson {
        let keys = hson.cache.keys();
        let mut hson_cache_copy = hson.cache.clone();

        for key in keys {
            if let Some(mut hson_cache) = hson_cache_copy.get_mut(key) {
                match self.cache.get_mut(key) {
                    Some(self_cache) => {
                        self_cache.append(&mut hson_cache);

                        let nodes = self.nodes.clone();
                        self_cache.sort_by(|a, b| {
                            let first = &nodes[a];
                            let second = &nodes[b];

                            first.instance.cmp(&second.instance)
                        });
                    },
                    None => {
                        self.cache.insert(key.clone(), hson_cache.clone());
                    }
                }
            }
        }



        hson
    }

    /// Right push existing nodes instance, key and value
    fn right_push_instances (&mut self, start: u64, distance: u64, data_size: usize) -> Result<(), Error> {
        let l = self.indexes.len();
        let mut i = 0;
        let root_id = self.get_root();

        if let Some(node) = self.nodes.get_mut(&root_id) {
            node.value[1] += data_size;
        }

        loop {
            let key = &self.indexes[i];
            if let Some(n) = self.nodes.get_mut(key) {
                if n.instance >= start {
                    n.instance += distance;
                    n.key[0] += data_size;
                    n.key[1] += data_size;
                    n.value[0] += data_size;
                    n.value[1] += data_size;
                }
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
    fn remove_from_nodes (&mut self, parent_id: u64, node_id: u64) {
        // Remove the node from its parent childs
        if let Some(n) = self.nodes.get_mut(&parent_id) {
            if let Some(i) = n.childs.iter().position(|s| s == &node_id) {
                n.childs.remove(i);
            }
        };

        // Remove the node from the existing indexes
        if let Some(i) = self.indexes.iter().position(|s| s == &node_id) {
            self.indexes.remove(i);
        };

        // Remove all node childs from the existing nodes
        match self.get_all_childs(node_id) {
            Ok(childs) => {
                for child in childs {
                    self.nodes.remove(&child);
                    if let Some(i) = self.indexes.iter().position(|s| s == &child) {
                        self.indexes.remove(i);
                    };
                }
            },
            Err(_e) => {}
        };

        self.nodes.remove(&node_id);
    }

    /// Remove a node from the cache
    fn remove_from_cache (&mut self, key: &str, node_id: u64) {
        if let Some(v) = self.cache.get_mut(key) {
            let mut to_remove = -1;
            for (idx, id) in v.iter().enumerate() {
                if id == &node_id {
                    to_remove = idx as i32;
                    break;
                }
            }

            if to_remove >= 0 {
                v.remove(to_remove as usize);
            }

            if v.is_empty() {
                self.cache.remove_entry(key);
            }
        }
    }

    /// Left push existing nodes instance, key and value
    fn left_push_instances (&mut self, start: u64, distance: u64, data_size: usize) -> Result<(), Error> {
        let l = self.indexes.len();
        let mut i = 0;
        let root_id = self.get_root();

        if let Some(node) = self.nodes.get_mut(&root_id) {
            node.value[1] -= data_size;
        }

        loop {
            let key = &self.indexes[i];
            if let Some(n) = self.nodes.get_mut(key) {
                if n.instance >= start {
                    n.instance -= distance;
                    n.key[0] -= data_size;
                    n.key[1] -= data_size;
                    n.value[0] -= data_size;
                    n.value[1] -= data_size;
                }
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
    fn insert_comma (&mut self, node_id: u64, parent_id: u64) {
        if let Some(node) = self.nodes.get(&node_id) {
            let mut instance = node.instance;
            let pos = node.value[1] + 1;
            let mut i = 0;

            if !node.childs.is_empty() {
                if let Some(child) = self.nodes.get(&node.childs[node.childs.len() - 1]) {
                    instance = child.instance;
                }
            }

            self.data.insert(pos, ',');
            loop {
                let idx = &self.indexes[i];
                if let Some(n) = self.nodes.get_mut(idx) {
                    if i == 0 {
                        n.value[1] += 1;
                    } else if n.instance > instance {
                        n.key[0] += 1;
                        n.key[1] += 1;
                        n.value[0] += 1;
                        n.value[1] += 1;
                    } else if n.id == parent_id {
                        n.value[1] += 1;
                    }
                }

                i += 1;
                if i >= self.indexes.len() {
                    break;
                }
            }
        };
    }

    /// Recursive method looking for nodes matching the query
    fn retrieve (&mut self, query: Vec<&str>) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();
        let mut tmp: Vec<(u64, u64)> = Vec::new();
        let mut i = (query.len() - 1) as i32;

        loop {
            if i as usize == query.len() - 1 {
                match self.cache.get(query[i as usize]) {
                    Some(v) => {
                        for uid in v {
                            if let Some(n) = self.nodes.get(uid) {
                                let n: (u64, u64) = (n.id, n.parent);
                                tmp.push(n);
                            }
                        }
                    },
                    None => return Ok(results)
                }
            } else {
                let mut res: Vec<(u64, u64)> = Vec::new();

                for map in &tmp {
                    let parent = &map.1;

                    if let Some(node) = self.nodes.get(parent) {
                        let key = self.get_node_key(node);

                        if key == query[i as usize] {
                            let n: (u64, u64) = (map.0, node.parent);
                            res.push(n);
                        }
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
            if c == DOUBLE_QUOTES {
                if !in_string {
                    in_string = true;
                } else if previous != BACKSLASH {
                    in_string = false;
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
    fn caching (&mut self, key: String, node_id: u64) {
        match self.cache.get_mut(&key) {
            Some(v) => v.push(node_id),
            None => {
                let mut ids = Vec::new();
                ids.push(node_id);

                self.cache.insert(key, ids);
            }
        }
    }

    /// Deduplicate vector's values
    fn unique (&self, v: &[u64]) -> Vec<u64> {
        let mut results = Vec::new();

        for value in v {
            if !results.contains(value) {
                results.push(*value);
            }
        }

        results
    }

    fn controls_count (&mut self, c: char, previous: char) {
        if c == OPEN_CURLY {
            self.controls.curly_brackets += 1;
        } else if c == CLOSE_CURLY {
            self.controls.curly_brackets -= 1;
        } else if c == DOUBLE_QUOTES {
            if self.controls.double_quotes > 0 && previous != BACKSLASH {
                self.controls.double_quotes = 0;
            } else {
                self.controls.double_quotes = 1;
            }
        } else if c == OPEN_ARR {
            self.controls.square_brackets += 1;
        } else if c == CLOSE_ARR {
            self.controls.square_brackets -= 1;
        }
    }

    fn validate (&self) -> Result<(), Error> {
        for value in self.nodes.values() {
            if value.opened {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid instance `{}`", value.instance));
                return Err(e);
            }
        }

        Ok(())
    }
}


impl Default for Hson {
    fn default () -> Self {
        Self::new()
    }
}


pub trait Query {
    fn query (&mut self, q: &str) -> Result<Vec<u64>, Error>;

    fn query_nodes (&mut self, q: &str) -> Result<Vec<&Node>, Error>;

    fn query_on (&mut self, node_id: u64, q: &str, recursive: bool) -> Result<Vec<u64>, Error>;

    fn query_on_nodes (&mut self, node: &Node, q: &str, recursive: bool) -> Result<Vec<&Node>, Error>;
}

impl Query for Hson {
    /// Public method to query the data
    fn query (&mut self, q: &str) -> Result<Vec<u64>, Error> {
        let parts: Vec<&str> = q.split(' ').collect();
        let results = self.retrieve(parts)?;

        Ok(results)
    }

    /// Same as `query` but return nodes structures instead of their ids
    fn query_nodes (&mut self, q: &str) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(' ').collect();

        let ids = self.retrieve(parts)?;
        for uid in &ids {
            results.push(&self.nodes[uid]);
        }

        Ok(results)
    }

    /// Same as `query` but constrain the search in the provided node's childs only
    fn query_on (&mut self, node_id: u64, q: &str, recursive: bool) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(' ').collect();

        let ids = self.retrieve(parts)?;
        for uid in ids {
            if recursive {
                if self.is_descendant(node_id, uid) {
                    results.push(uid);
                }
            } else if let Some(n) = self.nodes.get(&uid) {
                if n.parent == node_id {
                    results.push(uid);
                }
            }
        }

        Ok(results)
    }

    /// Same as `query_on` but return nodes structures instead of their ids
    fn query_on_nodes (&mut self, node: &Node, q: &str, recursive: bool) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = q.split(' ').collect();

        let ids = self.retrieve(parts)?;
        for uid in ids {
            if recursive {
                if self.is_descendant(node.id, uid) {
                    results.push(&self.nodes[&uid]);
                }
            } else if let Some(n) = self.nodes.get(&uid) {
                if n.parent == node.id {
                    results.push(&self.nodes[&uid]);
                }
            }
        }

        Ok(results)
    }
}


pub trait Ops {
    fn insert (&mut self, node_id: u64, idx: usize, s: &str) -> Result<(), Error>;

    fn remove (&mut self, node_id: u64) -> Result<(), Error>;
}

impl Ops for Hson {
    /// Insert an hson slice
    fn insert (&mut self, node_id: u64, insert_pos: usize, data_to_insert: &str) -> Result<(), Error> {
        let mut slice_range = 0;

        match self.nodes.get(&node_id) {
            Some(node) => {
                let mut t = self.clean(&data_to_insert);
                // Start instances count (for new_slice method) at the provided node instance number
                // Subtract 1 to take care of the root instance in the new hson slice
                let mut start_instance = node.instance - 1;
                // From what instance the existing nodes should be pushed to the right
                let mut start = node.instance + 1;
                // From what position should the new slice should be inserted in the existing data
                let mut start_idx = node.value[0];
                let parent_id = node.id;

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
                            if !c.childs.is_empty() {
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

                if !node.childs.is_empty() {
                    // Insert a comma if the inserting position is not the last one and there's not already one
                    // When inserting in the middle of childs add the comma and push the position of all following nodes
                    if insert_pos < node.childs.len() && t[t.len() - 2] != COMMA {
                        t.insert(t.len() - 1, ',');
                    } else if insert_pos >= node.childs.len() {
                        let last_child_uid = node.childs[node.childs.len() - 1];
                        let current_uid = node.id;
                        self.insert_comma(last_child_uid, current_uid);
                        start_idx += 1;
                    }
                }

                // Parsing the new slice occurs only here to allow borrowing as there's no more use of the node variable
                self.id_count += 1;
                let s: String = t.into_iter().collect();
                let s = s.as_str();
                let mut hson = Hson::new_slice(self.id_count, start_instance);
                hson.parse(s)?;

                let root_id = hson.get_root();
                if let Some(n) = hson.nodes.get(&root_id) {
                    slice_range = n.value[1] - n.value[0];
                }

                let num_keys = hson.nodes.keys().len() as u64;
                // How much does the existing nodes must be pushed
                let distance = num_keys - 1;
                // Data size subtracting the root's curly bracket chars
                let data_size = hson.data.len() - 2;

                self.id_count += num_keys;
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

        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.value[1] += slice_range;
        };

        if let Some(c) = self.callback {
            c(Event::Insert, node_id);
        }

        Ok(())
    }

    /// Remove a node and all its childs
    fn remove (&mut self, node_id: u64) -> Result<(), Error> {
        match self.nodes.get(&node_id) {
            Some(node) => {
                let key = self.get_node_key(node);
                let childs = self.get_all_childs(node_id)?;
                let instances_range = childs.len() + 1;
                let start_instance = node.instance + childs.len() as u64 + 1;
                let parent_id = node.parent;
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
                    if let Some(n) = self.nodes.get(&child) {
                        let key = self.get_node_key(n);

                        if !key.is_empty() {
                            self.remove_from_cache(&key, child);
                        }
                    }
                }

                self.left_push_instances(start_instance, instances_range as u64, data_size)?;
                self.remove_from_data(data_start_pos, data_end_pos);
                self.remove_from_nodes(parent_id, node_id);
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", node_id));
                return Err(e);
            }
        }

        if let Some(c) = self.callback {
            c(Event::Remove, node_id);
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
            if c == &DOUBLE_QUOTES {
                if !in_string {
                    in_string = true;
                } else if previous != &BACKSLASH {
                    in_string = false;
                }
            } else if c == &COMMA && !in_string {
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
        match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None
        }
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
            let mut previous_instance = self.instances - self.indexes.len() as u64;

            loop {
                for (key, value) in &self.nodes {
                    let node = &self.nodes[key];

                    if node.instance == previous_instance + 1 {
                        println!("{} : {:?}", self.get_node_key(value), value);
                        previous_instance += 1;
                    }
                }

                if previous_instance >= self.instances as u64 {
                    break;
                }
            }
        } else {
            for value in self.nodes.values() {
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
                self.controls_count(c, previous);
                let in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != BACKSLASH;

                if !in_string {
                    match self.controls.chars.iter().position(|&s| s == c) {
                        Some(_) => {
                            match c {
                                OPEN_CURLY => {
                                    in_array = false;
                                    print!("{}", c);
                                    indent += 1;
                                    println!();
                                    for _t in 0..indent {
                                        print!("\t");
                                    }
                                },
                                CLOSE_CURLY => {
                                    indent -= 1;
                                    println!();
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
                                        println!();
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
            if let Some(v) = self.cache.get(key) {
                for uid in v {
                    println!("\t{}", &uid);
                }
            }
        }
    }
}


pub trait Search {
    fn search (&mut self, query: &str) -> Result<Vec<u64>, Error>;

    fn search_in (&mut self, node_id: u64, query: &str) -> Result<Vec<u64>, Error>;
}

impl Search for Hson {
    /// Enhanced queries
    // standard search: div p
    // no recursive search: div>p
    // multiple search: div p|ul|article
    // equality search: div p attrs id='12'
    fn search (&mut self, query: &str) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();
        let q = self.format_query(query);
        let root_id = self.get_root();
        let first = true;

        // Add the root node in the results list for first lookup
        results.push(root_id);
        for part in q {
            if part.contains('>') {
                results = self.find_childs(&part, &results, first)?;
            } else if part.contains('|') {
                results = self.find_multiple_childs(&part, &results)?;
            } else {
                results = self.find_descendants(&part, &results)?;
            }
        }

        // TODO: IMPROVE SEARCH TO AVOID DUPLICATE ENTRIES
        results = self.unique(&results);

        Ok(results)
    }

    fn search_in (&mut self, node_id: u64, query: &str) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();
        let q = self.format_query(query);
        let first = true;

        // Add the root node in the results list for first lookup
        results.push(node_id);
        for part in q {
            if part.contains('>') {
                results = self.find_childs(&part, &results, first)?;
            } else if part.contains('|') {
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

    fn find_descendants (&mut self, query: &str, existing: &[u64]) -> Result<Vec<u64>, Error>;

    fn find_childs (&mut self, query: &str, existing: &Vec<u64>, first: bool) -> Result<Vec<u64>, Error>;

    fn find_multiple_childs (&mut self, query: &str, existing: &[u64]) -> Result<Vec<u64>, Error>;

    fn filter_equality_childs (&mut self, query: &str, results: &[u64]) -> Result<Vec<u64>, Error>;
}

impl SearchUtils for Hson {
    fn format_query (&self, query: &str) -> Vec<String> {
        let mut result = Vec::new();
        let q = self.clean_query(query);
        let mut in_string = false;
        let mut previous = ' ';
        let mut item = String::from("");

        for c in q {
            if c == QUOTE {
                if !in_string {
                    in_string = true;
                } else if previous != BACKSLASH {
                    in_string = false;
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
            if c == QUOTE {
                if !in_string {
                    in_string = true;
                } else if previous != BACKSLASH {
                    in_string = false;
                }
            }

            if in_string {
                string_array.push(c);
            } else if c == '>' || c == '=' || c == '|' {
                let l = string_array.len();
                if l > 0 && string_array[l - 1] == ' ' {
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

    fn find_descendants (&mut self, query: &str, existing: &[u64]) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();

        for r in existing {
            if query.contains('=') {
                let parts: Vec<&str> = query.split('=').collect();
                let mut t = self.query_on(*r, parts[0], true)?;
                t = self.filter_equality_childs(query, &t)?;
                results.append(&mut t);
            } else {
                let mut t = self.query_on(*r, query, true)?;
                results.append(&mut t);
            }
        }

        Ok(results)
    }

    fn find_childs (&mut self, query: &str, existing: &Vec<u64>, mut first: bool) -> Result<Vec<u64>, Error> {
        let mut results = existing.clone();
        let mut elements: Vec<&str> = query.split('>').collect();

        if elements[0].is_empty() {
            elements.remove(0);
            first = false;
        }

        // Loop on each element of the query
        for elm in elements {
            let mut res = Vec::new();

            // And look for those query elements in existing results
            for r in results {
                if elm.contains('=') {
                    let parts: Vec<&str> = elm.split('=').collect();
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

    fn find_multiple_childs (&mut self, query: &str, existing: &[u64]) -> Result<Vec<u64>, Error> {
        let elements: Vec<&str> = query.split('|').collect();
        let mut results = Vec::new();

        for elm in elements {
            for r in existing {
                if elm.contains('=') {
                    let parts: Vec<&str> = elm.split('=').collect();
                    let mut t = self.query_on(*r, parts[0], true)?;
                    t = self.filter_equality_childs(elm, &t)?;
                    results.append(&mut t);
                } else {
                    let mut t = self.query_on(*r, elm, true)?;
                    results.append(&mut t);
                }
            }
        }

        Ok(results)
    }

    fn filter_equality_childs (&mut self, query: &str, existing: &[u64]) -> Result<Vec<u64>, Error> {
        let mut results = Vec::new();
        let parts: Vec<&str> = query.split('=').collect();
        let chars: Vec<char> = parts[1].chars().collect();
        let equality = chars[1..chars.len()-1].iter().cloned().collect::<String>();
        let mut patterns = Vec::new();

        patterns.push(equality.as_str());
        if patterns[0].contains('|') {
            patterns = patterns[0].split('|').collect();
        }

        for res in existing {
            if let Some(node) = self.nodes.get(res) {
                for pattern in &patterns {
                    let value = self.get_node_value(node);

                    if value == pattern.trim() {
                        results.push(res.clone());
                    }
                }
            }
        }

        Ok(results)
    }
}


impl Iterator for Hson {
    type Item = u64;

    fn next (&mut self) -> Option<u64> {
        if !self.indexes.is_empty() && self.iter_count < self.indexes.len() {
            let id = self.indexes[self.iter_count];
            self.iter_count += 1;
            Some(id)
        } else {
            self.iter_count = 0;
            None
        }
    }
}
