#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;

extern crate hson;
use hson::{ Hson, Query, Ops };


lazy_static! {
    static ref SHORT_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/small.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref LONG_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/long.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };
}

#[test]
fn can_parse () {
    let mut hson = Hson::new();
    assert_eq!(hson.parse(&SHORT_DATA).unwrap(), ());
}

#[test]
#[should_panic]
fn cant_parse () {
    let data = r#"{
            "div": {
                "class": [],
                "text": "World
            },
            "ul": {
                "class": ["active","test"]
            }
        }"#;

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();
}

#[test]
fn has_nodes_number () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 18);
}

#[test]
#[should_panic]
fn invalid_chars () {
    let data = r#"{
            "div": {
                class: [],
                "text": "World
            },
            "ul": {
                "class": ["active","test"]
            }
        }"#;

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();
}

#[test]
fn query_retrieve_elements () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("attrs").unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn query_retrieve_in_node () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div p").unwrap();
    let childs_results = hson.query_on(&results[0], "span", true).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn query_retrieve_in_node_only () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div").unwrap();
    let childs_results = hson.query_on(&results[0], "attrs", false).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn insertion () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div p attrs").unwrap();
    assert_eq!(results.len(), 1);

    let child = r#"{
                        "class": ["active", "item"],
                        "name": "foo"
                    }"#;

    assert_eq!(hson.insert(&results[0], 0, child).unwrap(), ());
    assert_eq!(hson.indexes.len(), 20);
    assert_eq!(hson.nodes.keys().len(), 20);
}

#[test]
fn deletion () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("p").unwrap();
    assert_eq!(results.len(), 2);

    assert_eq!(hson.remove(&results[0]).unwrap(), ());
    assert_eq!(hson.indexes.len(), 14);
    assert_eq!(hson.nodes.keys().len(), 14);
}