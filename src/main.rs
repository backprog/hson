extern crate uuid;

use std::collections::HashMap;
use std::vec::Vec;
use std::iter::FromIterator;
use std::io::{ ErrorKind, Error };

use uuid::Uuid;


const OPEN_CURLY: char = '{';
const CLOSE_CURLY: char = '}';
const OPEN_ARR: char = '[';
const CLOSE_ARR: char = ']';
const DOUBLE_QUOTES: char = '"';
const COLONS: char = ':';
const COMMA: char = ',';

#[derive(PartialEq, Debug)]
enum Kind {
    Node,
    Array,
    Integer,
    Float,
    String,
    Json
}

#[derive(Debug)]
struct Node {
    root: bool,
    kind: Kind,
    parent: String,
    childs: Vec<String>,
    key: [usize; 2],
    value: [usize; 2],
    id: String,
    opened: bool,
    json: bool,
    instance: u32
}

struct Controls {
    chars: [char; 7],
    curly_brackets: u16,
    square_brackets: u16,
    double_quotes: u16
}

pub struct Hson {
    data: Vec<char>,
    nodes: HashMap<String, Node>,
    indexes: Vec<String>,
    controls: Controls,
    instances: u32
}

impl Hson {
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
            }
        };

        hson
    }

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
            }
        };

        hson
    }

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

            // Is current char is a double quotes closing a string
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

            // Is current char is a control character
            match self.controls.chars.iter().position(|&s| s == c && !in_string) {
                Some(_) => {
                    self.controls_count(&c, &previous);

                    // Is current char position is before colons
                    if c != DOUBLE_QUOTES && !in_string && !in_array {
                        before_colons = true;
                    }

                    if c == COLONS || (c == OPEN_CURLY && i == 0) {
                        if c == COLONS {
                            before_colons = false;
                        }

                        let uid = Uuid::new_v4().to_string();
                        let root = if i == 0 { true } else { false };
                        let parent = if i == 0 { String::from("") } else { self.indexes[self.indexes.len() - 1].clone() };
                        let key = if i == 0 { [0, 0] } else { self.get_node_key_position(i, &data)? };
                        let mut kind = if i == 0 { Kind::Node } else { self.get_node_kind(i, &data)? };
                        let mut json = false;

                        // If kind is json, remove json tag, switch back to String kind and mark the node as json
                        if kind == Kind::Json {
                            l = self.remove_type(i, &mut data, "json");
                            kind = Kind::String;
                            json = true;
                        }

                        self.instances += 1;
                        self.indexes.push(uid.clone());
                        self.nodes.insert(uid.clone(), Node {
                            root,
                            kind,
                            parent,
                            childs: Vec::new(),
                            key,
                            value: [i + 1, data.len()],
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
                None => {}
            }

            previous = c;
            i += 1;

            if i >= l {
                break;
            }
        }

        self.data = data;

        Ok(())
    }

    fn remove_type (&self, pos: usize, data: &mut Vec<char>, kind: &str) -> usize {
        if kind == "json" {
            data.splice(pos+1..pos+7, vec!());
        }

        data.len()
    }

    fn get_node_key (&self, n: &Node) -> String {
        let mut key = String::from("");
        let start = n.key[0] + 1;
        let end = n.key[1];

        for i in start..end {
            key.push(self.data[i]);
        }

        key
    }

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
                    if data[n] == ',' {
                        let v: String = Vec::from_iter(data[i..n-1].iter().cloned()).into_iter().collect();

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

    fn clean (&self, s: &str) -> Vec<char> {
        let mut string_array = Vec::new();

        for (i, c) in s.chars().enumerate() {
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

    //DEBUG
    fn print_nodes (&self) {
        for (key, value) in &self.nodes {
            println!("{} : {:?}", self.get_node_key(value), value);
        }
    }

    fn print_data (&self) {
        let s: String = self.data.iter().collect();
        println!("{}", &s);
    }
}

trait Query {
    fn query (&self, q: &str) -> Result<Vec<String>, Error>;

    fn query_nodes (&self, q: &str) -> Result<Vec<&Node>, Error>;

    fn find (&self, elements: &Vec<String>, mut query: Vec<&str>, first: bool) -> Result<Vec<String>, Error>;

    fn get_all_childs (&self, node: &Node) -> Result<Vec<String>, Error>;
}

impl Query for Hson {
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

    fn find (&self, elements: &Vec<String>, mut query: Vec<&str>, first: bool) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();

        if query.len() > 0 {
            let current = if first { "" } else { query.remove(0) };
            let l = query.len();

            for uid in elements {
                let mut q = query.clone();

                match self.nodes.get(uid) {
                    Some(n) => {
                        if current.starts_with('#') {

                        } else if current.starts_with('.') {

                        } else {
                            let key = if first { String::from("") } else { self.get_node_key(&n) };
                            if &key == current {
                                if l == 0 {
                                    results.push(uid.clone());
                                }
                            } else {
                                if !first {
                                    q.insert(0, current);
                                }
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

    fn get_all_childs (&self, node: &Node) -> Result<Vec<String>, Error> {
        let mut results = Vec::new();

        if node.childs.len() > 0 {
            results.append(&mut node.childs.clone());

            for uid in &node.childs {
                match self.nodes.get(uid) {
                    Some(n) => {
                        let mut res = self.get_all_childs(n)?;
                        results.append(&mut res);
                    },
                    None => {}
                }
            }
        }

        Ok(results)
    }
}

trait Ops {
    fn insert (&mut self, uid: &String, idx: usize, s: &str) -> Result<(), Error>;

    fn remove (&mut self, uid: &String) -> Result<(), Error>;

    fn insert_into_data (&mut self, hson: Hson, start: usize) -> Hson;

    fn insert_into_nodes (&mut self, parent_id: String, start_idx: usize, hson: Hson) -> Hson;

    fn right_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error>;

    fn remove_from_data (&mut self, node: &Node);

    fn remove_from_nodes (&mut self, node: &Node);

    fn left_push_instances (&mut self, start: u32, distance: u32, data_size: usize) -> Result<(), Error>;
}

impl Ops for Hson {
    fn insert (&mut self, uid: &String, idx: usize, s: &str) -> Result<(), Error> {
        match self.nodes.get(uid) {
            Some(node) => {
                let mut hson = Hson::new_slice(node.instance - 1);
                hson.parse(s)?;

                let num_keys = hson.nodes.keys().len() as u32;
                let start = node.instance + 1;
                let distance = num_keys - 1;
                let start_idx = node.value[0] + 1;
                let parent_id = node.id.clone();
                let data_size = hson.data.len() - 2;

                self.right_push_instances(start, distance, data_size);
                hson = self.insert_into_data(hson, start_idx);
                hson = self.insert_into_nodes(parent_id, start_idx, hson);

                Ok(())
            },
            None => {
                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", uid));
                return Err(e);
            }
        }
    }

    fn remove (&mut self, uid: &String) -> Result<(), Error> {
//        match self.nodes.get(&uid) {
//            Some(node) => {
//                let num_keys = hson.nodes.keys().len() as u32;
//                let start = node.instance + 1;
//                let distance = num_keys - 1;
//                let start_idx = node.value[0] + 1;
//                let node_id = node.id.clone();
//                let data_size = node.value[1] - node.key[0];
//
//                self.remove_from_data(node);
//                self.remove_from_nodes(node);
//                self.left_push_instances(start, distance, data_size)?;
//            },
//            None => {
//                let e = Error::new(ErrorKind::InvalidData, format!("Invalid uid {}", uid));
//                return Err(e);
//            }
//        }

        Ok(())
    }

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

    fn insert_into_nodes (&mut self, parent_id: String, start_idx: usize, mut hson: Hson) -> Hson {
        let mut root_id = String::from("");
        let mut pos = start_idx;
        let mut previous_key = [0, 0];

        for (i, key) in hson.indexes.iter().enumerate() {
            match hson.nodes.remove_entry(key) {
                Some((k, mut node)) => {
                    if node.root {
                        root_id = node.id;
                    } else {
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

                        node.key[0] = pos;
                        node.key[1] = node.key[0] + key_diff;
                        node.value[0] = node.key[1] + 1;
                        node.value[1] = node.key[1] + value_diff;
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

        Ok(())
    }

    fn remove_from_data (&mut self, node: &Node) {
        let begin = node.key[0];
        let end = node.value[1];

        self.data.splice(begin..end, vec!());
    }

    fn remove_from_nodes (&mut self, node: &Node) {
        match self.nodes.get_mut(&node.parent) {
            Some(n) => {
                match n.childs.iter().position(|s| s == &node.id) {
                    Some(i) => {
                        n.childs.remove(i);
                    },
                    None => {}
                }
            },
            None => {}
        };

        match self.indexes.iter().position(|s| s == &node.id) {
            Some(i) => {
                self.indexes.remove(i);
            },
            None => {}
        };
    }

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

        Ok(())
    }
}


fn main() {
    let data = r#"{
    	"div": {
            "attrs": {
                "class": [""],
                "onClick": "doSomething",
                "rel": <json>"{\"div\":[[\"abc\"],[\"cde\",\"fgh\"]]}"
            },
            "div": {
                "p":{
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

    hson.parse(data).unwrap();
    let results = hson.query("div p").unwrap();

    println!("\n{:?}\n", results);
    let child = r#"{
                        "i": {
                            "class": [],
                            "text": "World"
                        },
                        "ul": {
                            "class": ["active","test"]
                        }
                    }"#;

    hson.insert(&results[1], 0, child).unwrap();
    hson.print_nodes();
    hson.print_data();
    let results = hson.query("p").unwrap();
    println!("\n{:?}\n", results);
    match hson.nodes.get(&results[0]) {
        Some(node) => {
            let res = hson.get_all_childs(node).unwrap();
            println!("\n{:?}\n", res);
        },
        None => {}
    };

    //hson.remove(&results[0]);
}
