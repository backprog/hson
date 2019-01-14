#![allow(unused_assignments)]
#![allow(dead_code)]

extern crate uuid;

use std::collections::HashMap;
use std::vec::Vec;
use std::iter::FromIterator;
use std::io::{ ErrorKind, Error };
use std::time::{ Instant };

use uuid::Uuid;


const OPEN_CURLY: char = '{';
const CLOSE_CURLY: char = '}';
const OPEN_ARR: char = '[';
const CLOSE_ARR: char = ']';
const DOUBLE_QUOTES: char = '"';
const COLONS: char = ':';
const COMMA: char = ',';


/// Node types
#[derive(PartialEq, Clone, Debug)]
pub enum Kind {
    Node,
    Array,
    Integer,
    Float,
    String,
    Json
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
    pub json: bool,
    pub instance: u32
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
    vec_it: Vec<String>
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
                chars: ['{', '}', '[', ']', ':', '"', ','],
                curly_brackets: 0,
                square_brackets: 0,
                double_quotes: 0
            },
            process_start: Instant::now(),
            vec_it: Vec::new()
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
                chars: ['{', '}', '[', ']', ':', '"', ','],
                curly_brackets: 0,
                square_brackets: 0,
                double_quotes: 0
            },
            process_start: Instant::now(),
            vec_it: Vec::new()
        };

        hson
    }

    /// Parse an hson string
    pub fn parse (&mut self, s: &str) -> Result<(), Error> {
        let mut data: Vec<char> = self.clean(&s);
        let mut previous = ' ';
        let mut before_colons = true;
        let mut string_just_closed = false;
        let mut l = data.len();
        let mut i = 0;

        loop {
            let c = data[i];

            // If structure does not start with curly bracket throw error
            if i == 0 && c != OPEN_CURLY {
                let e = Error::new(ErrorKind::InvalidData, "Invalid character at 0");
                return Err(e);
            }

            let in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != '\\';
            let in_array = self.controls.square_brackets > 0;

            // Is current char a double quotes closing a string
            if self.controls.double_quotes > 0 && c == DOUBLE_QUOTES && previous != '\\' {
                string_just_closed = true;
            } else {
                string_just_closed = false;
            }

            // DEBUG
            /*
            println!("CHAR: {}", &c);
            println!("IN_STRING: {}", &in_string);
            println!("IN_ARRAY: {}", &in_array);
            println!("BEFORE_COLONS: {}", &before_colons);
            println!("STRING_CLOSED: {}", &string_just_closed);
            */

            // Is current char a control character
            match self.controls.chars.iter().position(|&s| s == c && !in_string) {
                Some(_) => {
                    self.controls_count(&c, &previous);

                    // Is current char position is before colons
                    if c != DOUBLE_QUOTES && !in_string && !in_array {
                        before_colons = true;
                    }

                    // If colons char is encountered or the first opening curly bracket
                    if c == COLONS || (c == OPEN_CURLY && i == 0) {
                        if c == COLONS {
                            before_colons = false;
                        }

                        let uid = Uuid::new_v4().to_string();
                        let root = i == 0;
                        let parent = if i == 0 { String::from("") } else { self.get_previous_opened_node(self.indexes.len())? };
                        let key = if i == 0 { [0, 0] } else { self.get_node_key_position(i, &data)? };
                        let value = if i == 0 { [i, data.len()] } else { [i + 1, data.len()] };
                        let mut kind = if i == 0 { Kind::Node } else { self.get_node_kind(i, &data)? };
                        let mut json = false;

                        // If kind is json, remove json tag, switch back to String kind and mark the node as json
                        // Note: Add support for other tag formats
                        if kind == Kind::Json {
                            l = self.remove_type(i, &mut data, "json");
                            kind = Kind::String;
                            json = true;
                        }

                        // Insert the new node
                        self.instances += 1;
                        self.indexes.push(uid.clone());
                        self.nodes.insert(uid.clone(), Node {
                            root,
                            kind,
                            parent,
                            childs: Vec::new(),
                            key,
                            value,
                            id: uid.clone(),
                            opened: true,
                            json,
                            instance: self.instances
                        });

                        if i > 0 {
                            // Get previous opened node and insert current node as one of its childs
                            let prev_node_uid = self.get_previous_opened_node(self.nodes.len() - 1)?;
                            match self.nodes.get_mut(&prev_node_uid) {
                                Some(n) => n.childs.push(uid),
                                None => {
                                    let e = Error::new(ErrorKind::Other, format!("Parent node cannot be retrieved at {}", i));
                                    return Err(e);
                                }
                            };
                        }
                    }

                        // Closing controls. Get previous opened node and close it
                        else if c == CLOSE_CURLY || c == CLOSE_ARR || (c == DOUBLE_QUOTES && string_just_closed && !before_colons && previous != '\\') {
                            let prev_node_uid = self.get_previous_node(i, &c)?;

                            match self.nodes.get_mut(&prev_node_uid) {
                                Some(n) => {
                                    n.opened = false;
                                    n.value = [n.value[0], i];
                                },
                                None => {
                                    let e = Error::new(ErrorKind::Other, format!("Node cannot be closed at {}", i));
                                    return Err(e);
                                }
                            };
                        }
                },
                None => {
                    if !in_string && before_colons {
                        let e = Error::new(ErrorKind::InvalidData, format!("Invalid character {} at {}", c, i));
                        return Err(e);
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
        self.fill_iterator();

        self.validate()
    }

    /// Retrieve a node key
    pub fn get_node_key (&self, n: &Node) -> String {
        let mut key = String::from("");
        let start = n.key[0] + 1;
        let end = n.key[1];

        for i in start..end {
            key.push(self.data[i]);
        }

        key
    }

    /// Retrieve a node value
    pub fn get_node_value (&self, n: &Node) -> String {
        let mut value = String::from("");
        let start = n.value[0] + 1;
        let end = n.value[1];

        for i in start..end {
            value.push(self.data[i]);
        }

        value
    }

    /// Remove format tags
    fn remove_type (&self, pos: usize, data: &mut Vec<char>, kind: &str) -> usize {
        if kind == "json" {
            data.splice(pos+1..pos+7, vec!());
        }

        data.len()
    }

    /// Retrieve a node key position
    fn get_node_key_position (&self, i: usize, data: &Vec<char>) -> Result<[usize; 2], Error> {
        let end = i - 1;
        let mut n = end;
        let mut k = [0, n];

        loop {
            n = if n > 0 { n - 1 } else {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid format at {}", i));
                return Err(e);
            };

            if data[n] == '"' {
                k[0] = n;
                break;
            }
        }

        Ok(k)
    }

    /// Retrieve a node type
    fn get_node_kind (&self, i: usize, data: &Vec<char>) -> Result<Kind, Error> {
        match data[i + 1] {
            '{' => Ok(Kind::Node),
            '[' => Ok(Kind::Array),
            '"' => Ok(Kind::String),
            '<' => {
                let slice: String = data[i + 2..i + 6].into_iter().collect();
                if &slice == "json" {
                    Ok(Kind::Json)
                } else {
                    let e = Error::new(ErrorKind::InvalidData, format!("Invalid type {} at {}", &slice, i + 1));
                    return Err(e);
                }
            },
            _ => {
                let mut n = i;
                let mut k = Kind::String;

                loop {
                    n += 1;
                    match self.controls.chars.iter().position(|&s| s == data[n]) {
                        Some(_) => {
                            let v: String = Vec::from_iter(data[i+1..n-1].iter().cloned()).into_iter().collect();

                            if v.parse::<i64>().is_ok() {
                                k = Kind::Integer;
                                break;
                            } else if v.parse::<f64>().is_ok() {
                                k = Kind::Float;
                                break;
                            } else {
                                let e = Error::new(ErrorKind::InvalidData, format!("Invalid value {} at {}", v, i));
                                return Err(e);
                            }
                        },
                        None => {}
                    }

                    if n > data.len() {
                        let e = Error::new(ErrorKind::InvalidData, format!("Invalid format at {}", i));
                        return Err(e);
                    }
                }

                Ok(k)
            }
        }
    }

    /// Retrieve the previous opened node of the same kind from the provided position in the hson string
    fn get_previous_node (&self, i: usize, c: &char) -> Result<String, Error> {
        let nodes = &self.nodes;
        let mut prev_node_uid = "";
        let mut l = self.indexes.len();

        loop {
            l = if l > 0 { l - 1 } else {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid format at {}", i));
                return Err(e);
            };

            match nodes.get(&self.indexes[l]) {
                Some(n) => {
                    let kind = match c {
                        &CLOSE_CURLY => Kind::Node,
                        &CLOSE_ARR => Kind::Array,
                        &DOUBLE_QUOTES => Kind::String,
                        _ => {
                            let e = Error::new(ErrorKind::InvalidData, format!("Invalid format at {}", i));
                            return Err(e);
                        }
                    };

                    if n.opened && n.kind == kind {
                        prev_node_uid = &n.id;
                        break;
                    }
                },
                None => {}
            };
        }

        Ok(prev_node_uid.to_string())
    }

    /// Same as `get_previous_node` but does not check the node kind
    fn get_previous_opened_node (&self, i: usize) -> Result<String, Error> {
        let nodes = &self.nodes;
        let mut prev_node_uid = "";
        let mut l = i;

        loop {
            l = if l > 0 { l - 1 } else {
                let e = Error::new(ErrorKind::Other, format!("Cannot retrieve previous opened node"));
                return Err(e);
            };

            match nodes.get(&self.indexes[l]) {
                Some(n) => {
                    if n.opened {
                        prev_node_uid = &n.id;
                        break;
                    }
                },
                None => {}
            };
        }

        Ok(prev_node_uid.to_string())
    }

    /// Clean a string of tab/newlines/spaces
    fn clean (&self, s: &str) -> Vec<char> {
        let mut string_array = Vec::new();

        for (_i, c) in s.chars().enumerate() {
            if c != ' ' && c != '\t' && c != '\r' && c != '\n' {
                string_array.push(c);
            }
        }

        string_array
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

    fn fill_iterator (&mut self) {
        self.vec_it.clear();
        let mut previous_instance = self.instances - self.indexes.len() as u32;

        loop {
            for (key, _value) in &self.nodes {
                let node = self.nodes.get(key).unwrap();

                if node.instance == previous_instance + 1 {
                    self.vec_it.push(node.id.clone());
                    previous_instance += 1;
                }
            }

            if previous_instance >= self.instances as u32 {
                break;
            }
        }
    }

    fn validate (&self) -> Result<(), Error> {
        for (_key, value) in &self.nodes {
            if value.opened {
                let mut key = self.get_node_key(value);

                if key.is_empty() {
                    key = String::from("root");
                }

                let e = Error::new(ErrorKind::InvalidData, format!("Invalid data at `{}`", key));
                return Err(e);
            }
        }

        Ok(())
    }
}


pub trait Query {
    fn query (&self, q: &str) -> Result<Vec<String>, Error>;

    fn query_nodes (&self, q: &str) -> Result<Vec<&Node>, Error>;

    fn find (&self, elements: &Vec<String>, query: Vec<&str>, first: bool) -> Result<Vec<String>, Error>;

    fn get_all_childs (&self, s: &String) -> Result<Vec<String>, Error>;

    fn get_all_node_childs (&self, node: &Node) -> Result<Vec<&Node>, Error>;
}

impl Query for Hson {
    /// Public method to query the data
    fn query (&self, q: &str) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();
        let mut set = Vec::new();
        let parts: Vec<&str> = q.split(" ").collect();

        set.push(self.indexes[0].clone());

        let ids = self.find(&set, parts, true)?;
        for uid in &ids {
            results.push(uid.clone());
        }

        Ok(results)
    }

    /// Same as query but return nodes structures instead of their ids
    fn query_nodes (&self, q: &str) -> Result<Vec<&Node>, Error> {
        let mut results = Vec::new();
        let mut set = Vec::new();
        let parts: Vec<&str> = q.split(" ").collect();

        set.push(self.indexes[0].clone());

        let ids = self.find(&set, parts, true)?;
        for uid in &ids {
            results.push(&self.nodes[uid]);
        }

        Ok(results)
    }

    /// Recursive method looking for nodes matching the query
    fn find (&self, elements: &Vec<String>, mut query: Vec<&str>, first: bool) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();

        if query.len() > 0 {
            let current = if first { "" } else { query.remove(0) };
            let l = query.len();

            // loop on provided nodes. First call of the method will get the root element
            for uid in elements {
                let mut q = query.clone();

                match self.nodes.get(uid) {
                    Some(n) => {
                        let key = if first { String::from("") } else { self.get_node_key(&n) };

                        // If the node's key match the query slice and it reached the last slice of the query
                        if &key == current {
                            if l == 0 {
                                results.push(uid.clone());
                            }
                        } else {
                            // If not the first call of the method and the key does not match, insert back the query slice
                            if !first {
                                q.insert(0, current);
                            }
                        }

                        let mut res = self.find(&n.childs, q, false)?;
                        results.append(&mut res);
                    },
                    None => {
                        let e = Error::new(ErrorKind::InvalidData, format!("Cannot find node uid {}", &uid));
                        return Err(e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get all childs of a node recursively
    fn get_all_childs (&self, s: &String) -> Result<Vec<String>, Error> {
        match self.nodes.get(s) {
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
                let e = Error::new(ErrorKind::InvalidData, format!("Cannot find node uid {}", s));
                return Err(e);
            }
        }
    }

    /// Same as `get_all_childs` but returning nodes structures instead of their ids
    fn get_all_node_childs (&self, node: &Node) -> Result<Vec<&Node>, Error> {
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
}


pub trait Ops {
    fn insert (&mut self, uid: &String, idx: usize, s: &str) -> Result<(), Error>;

    fn remove (&mut self, uid: &String) -> Result<(), Error>;

    fn insert_into_data (&mut self, hson: Hson, start: usize) -> Hson;

    fn insert_into_nodes (&mut self, parent_id: String, start_idx: usize, hson: Hson) -> Hson;

    fn right_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error>;

    fn remove_from_data (&mut self, begin: usize, end: usize);

    fn remove_from_nodes (&mut self, parent_id: &str, uid: &str);

    fn left_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error>;

    fn insert_comma (&mut self, uid: String, parent_uid: String);
}

impl Ops for Hson {
    /// Insert an hson slice
    fn insert (&mut self, uid: &String, idx: usize, s: &str) -> Result<(), Error> {
        let mut slice_range = 0;

        match self.nodes.get(uid) {
            Some(node) => {
                let mut t = self.clean(&s);
                // Start instances count (for new_slice method) at the provided node instance number
                // Subtract 1 to take care of the root instance in the new hson slice
                let mut start_instance = node.instance - 1;
                // From what instance the existing nodes should be pushed to the right
                let mut start = node.instance + 1;
                // From what position should the new slice should be inserted in the existing data
                let mut start_idx = node.value[0] + 1;
                let parent_id = node.id.clone();

                // If the inserting position in the node's childs is not the first one,
                // retrieve the new position based on the child values
                if idx > 0 {
                    let child_uid = match node.childs.get(idx - 1) {
                        Some(id) => id,
                        None => {
                            let e = Error::new(ErrorKind::InvalidData, format!("Invalid index {}", idx));
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
                    if idx < node.childs.len() && t[t.len() - 2] != COMMA {
                        t.insert(t.len() - 1, ',');
                    } else if idx >= node.childs.len() {
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

                match hson.nodes.get(&hson.indexes[0]) {
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
                hson = self.insert_into_nodes(parent_id, start_idx, hson);
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", uid));
                return Err(e);
            }
        };

        match self.nodes.get_mut(uid) {
            Some(node) => {
                node.value[1] += slice_range;
            },
            None => {}
        };

        self.fill_iterator();

        Ok(())
    }

    /// Remove a node and all its childs
    fn remove (&mut self, uid: &String) -> Result<(), Error> {
        match self.nodes.get(uid) {
            Some(node) => {
                let childs = self.get_all_childs(uid)?;
                let instances_range = childs.len() + 1;
                let start_instance = node.instance + childs.len() as u32 + 1;
                let parent_id = node.parent.clone();
                let data_start_pos = node.key[0];
                let mut data_end_pos = node.value[1] + 1;
                let mut data_size = node.value[1] - node.key[0] + 1;

                if self.data[data_end_pos] == COMMA {
                    data_end_pos += 1;
                    data_size += 1;
                }

                self.left_push_instances(start_instance, instances_range as u32, data_size)?;
                self.remove_from_data(data_start_pos, data_end_pos);
                self.remove_from_nodes(&parent_id, uid);
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", uid));
                return Err(e);
            }
        }

        self.fill_iterator();

        Ok(())
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
    fn insert_into_nodes (&mut self, parent_id: String, start_idx: usize, mut hson: Hson) -> Hson {
        let mut root_id = String::from("");
        let mut pos = start_idx;
        let mut previous_key = [0, 0];

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
                                Some(n) => n.childs.push(node.id.clone()),
                                None => {}
                            }
                        }

                        let key_diff = node.key[1] - node.key[0];
                        let value_diff = node.value[1] - node.value[0];

                        if previous_key[1] > 0 {
                            pos += node.key[0] - previous_key[1];
                        }

                        previous_key = node.key;

                        // Replace key and value position
                        node.key[0] = pos;
                        node.key[1] = node.key[0] + key_diff;
                        node.value[0] = node.key[1] + 2;
                        node.value[1] = node.value[0] + value_diff;
                        pos = node.key[1];

                        let idx = node.instance as usize;
                        self.indexes.insert(idx, key.clone());
                        self.nodes.insert(k, node);
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

        match self.nodes.get_mut(&self.indexes[0]) {
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

    /// Left push existing nodes instance, key and value
    fn left_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error> {
        let l = self.indexes.len();
        let mut i = 0;

        match self.nodes.get_mut(&self.indexes[0]) {
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
}


pub trait Debug {
    fn print_nodes (&self, sorted: bool);

    fn print_data (&mut self, pretty: bool);

    fn print_indexes (&self);

    fn print_process_time (&self);

    fn print_controls (&self);
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

            loop {
                let c = self.data[i];
                self.controls_count(&c, &previous);
                let in_string = self.controls.double_quotes > 0 && c != DOUBLE_QUOTES && previous != '\\';
                let in_array = self.controls.square_brackets > 0;

                match self.controls.chars.iter().position(|&s| s == c && !in_string) {
                    Some(_) => {
                        match c {
                            OPEN_CURLY => {
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
}


impl Iterator for Hson {
    type Item = String;

    fn next (&mut self) -> Option<String> {
        if !self.vec_it.is_empty() {
            let id = self.vec_it.remove(0);
            Some(id)
        } else {
            self.fill_iterator();
            None
        }
    }
}
